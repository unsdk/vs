use mlua::{Lua, Result as LuaResult, UserData, UserDataMethods, Value};
use scraper::{Html, Selector};

#[derive(Clone, Debug)]
struct HtmlSelection {
    fragments: Vec<String>,
    include_self_in_find: bool,
}

impl HtmlSelection {
    fn new(fragments: Vec<String>, include_self_in_find: bool) -> Self {
        Self {
            fragments,
            include_self_in_find,
        }
    }
}

impl UserData for HtmlSelection {
    fn add_methods<M: UserDataMethods<Self>>(methods: &mut M) {
        methods.add_method("find", |_, this, selector: String| {
            let selector = Selector::parse(&selector)
                .map_err(|error| mlua::Error::external(error.to_string()))?;
            let mut matches = Vec::new();
            for fragment in &this.fragments {
                let html = parse_wrapped_fragment(fragment);
                if this.include_self_in_find {
                    for element in html.select(&selector) {
                        matches.push(element.html());
                    }
                } else {
                    let root_selector = Selector::parse(root_selector_for_fragment(fragment))
                        .map_err(|error| mlua::Error::external(error.to_string()))?;
                    for root in html.select(&root_selector) {
                        for element in root.select(&selector) {
                            matches.push(element.html());
                        }
                    }
                }
            }
            Ok(HtmlSelection::new(matches, false))
        });
        methods.add_method("eq", |_, this, index: usize| {
            Ok(this
                .fragments
                .get(index)
                .map(|fragment| HtmlSelection::new(vec![fragment.clone()], false))
                .unwrap_or_else(|| HtmlSelection::new(Vec::new(), false)))
        });
        methods.add_method("first", |_, this, ()| {
            Ok(this
                .fragments
                .first()
                .map(|fragment| HtmlSelection::new(vec![fragment.clone()], false))
                .unwrap_or_else(|| HtmlSelection::new(Vec::new(), false)))
        });
        methods.add_method("last", |_, this, ()| {
            Ok(this
                .fragments
                .last()
                .map(|fragment| HtmlSelection::new(vec![fragment.clone()], false))
                .unwrap_or_else(|| HtmlSelection::new(Vec::new(), false)))
        });
        methods.add_method("html", |_, this, ()| Ok(this.fragments.join("")));
        methods.add_method("text", |_, this, ()| {
            let mut result = String::new();
            for fragment in &this.fragments {
                let html = parse_wrapped_fragment(fragment);
                let root_selector = Selector::parse(root_selector_for_fragment(fragment))
                    .map_err(|error| mlua::Error::external(error.to_string()))?;
                for root in html.select(&root_selector) {
                    for text in root.text() {
                        result.push_str(text);
                    }
                }
            }
            Ok(result)
        });
        methods.add_method("attr", |_, this, attribute: String| {
            for fragment in &this.fragments {
                let html = parse_wrapped_fragment(fragment);
                if let Ok(selector) = Selector::parse(root_selector_for_fragment(fragment)) {
                    if let Some(element) = html.select(&selector).next() {
                        if let Some(value) = element.value().attr(attribute.as_str()) {
                            return Ok(Some(value.to_string()));
                        }
                    }
                }
            }
            Ok(None::<String>)
        });
        methods.add_method("each", |_lua, this, callback: mlua::Function| {
            for (index, fragment) in this.fragments.iter().enumerate() {
                // Lua arrays are 1-based, so expose indices the same way.
                callback
                    .call::<()>((index + 1, HtmlSelection::new(vec![fragment.clone()], false)))
                    .map_err(|error| mlua::Error::external(error.to_string()))?;
            }
            Ok(Value::Nil)
        });
    }
}

pub(super) fn create_html_module(lua: &Lua) -> LuaResult<mlua::Function> {
    lua.create_function(|lua, ()| {
        let table = lua.create_table()?;
        table.set(
            "parse",
            lua.create_function(|_, input: String| Ok(HtmlSelection::new(vec![input], true)))?,
        )?;
        Ok(table)
    })
}

// `scraper` expects fragments to have valid parent elements, so wrap table-specific fragments
// into the smallest compatible document shape before running selectors against them.
pub(super) fn parse_wrapped_fragment(fragment: &str) -> Html {
    let wrapped = wrap_fragment(fragment);
    Html::parse_fragment(&wrapped)
}

fn root_selector_for_fragment(fragment: &str) -> &'static str {
    let normalized = fragment.trim_start().to_ascii_lowercase();
    if normalized.starts_with("<tr") {
        "table > tbody > tr"
    } else if normalized.starts_with("<td") || normalized.starts_with("<th") {
        "table > tbody > tr > *"
    } else if normalized.starts_with("<tbody") {
        "table > tbody"
    } else if normalized.starts_with("<thead") {
        "table > thead"
    } else {
        "vs-root > *"
    }
}

fn wrap_fragment(fragment: &str) -> String {
    let normalized = fragment.trim_start().to_ascii_lowercase();
    if normalized.starts_with("<tr") {
        format!("<table><tbody>{fragment}</tbody></table>")
    } else if normalized.starts_with("<td") || normalized.starts_with("<th") {
        format!("<table><tbody><tr>{fragment}</tr></tbody></table>")
    } else if normalized.starts_with("<tbody") || normalized.starts_with("<thead") {
        format!("<table>{fragment}</table>")
    } else {
        format!("<vs-root>{fragment}</vs-root>")
    }
}

#[cfg(test)]
mod tests {
    use std::error::Error;

    use mlua::Lua;
    use scraper::Selector;

    use super::parse_wrapped_fragment;
    use crate::module::{register_builtin_modules, set_package_paths};

    #[test]
    fn html_each_should_iterate_matches() -> Result<(), Box<dyn Error>> {
        let lua = Lua::new();
        let current_dir = std::env::current_dir()?;
        set_package_paths(&lua, &current_dir)?;
        register_builtin_modules(&lua, "vs-test/0.1.0", None)?;

        lua.load(
            r#"
            local html = require("html")
            local doc = html.parse("<div id='a'>A</div><div id='b'>B</div>")
            result = {}
            doc:find("div"):each(function(i, selection)
              table.insert(result, selection:attr("id"))
            end)
            "#,
        )
        .exec()?;

        let result: mlua::Table = lua.globals().get("result")?;
        let first: Option<String> = result.get(1)?;
        let second: Option<String> = result.get(2)?;
        assert_eq!(first.as_deref(), Some("a"));
        assert_eq!(second.as_deref(), Some("b"));

        Ok(())
    }

    #[test]
    fn html_find_should_match_archive_page_shape() -> Result<(), Box<dyn Error>> {
        let html = parse_wrapped_fragment(
            r#"
            <div id="archive">
              <div class="toggle" id="go1.26.0">
                <table class="downloadtable">
                  <tbody>
                    <tr>
                      <td>go1.26.0.darwin-arm64.tar.gz</td>
                      <td>Archive</td>
                      <td>macOS</td>
                      <td>ARM64</td>
                      <td>unused</td>
                      <td>abc123</td>
                    </tr>
                  </tbody>
                </table>
              </div>
            </div>
            "#,
        );

        let archive = Selector::parse("div#archive")?;
        let toggles = Selector::parse(".toggle")?;
        let rows = Selector::parse("table.downloadtable tbody tr")?;

        let archive_nodes = html.select(&archive).collect::<Vec<_>>();
        assert_eq!(archive_nodes.len(), 1);
        let toggle_nodes = archive_nodes[0].select(&toggles).collect::<Vec<_>>();
        assert_eq!(toggle_nodes.len(), 1);
        let row_nodes = toggle_nodes[0].select(&rows).collect::<Vec<_>>();
        assert_eq!(row_nodes.len(), 1);
        Ok(())
    }

    #[test]
    fn html_pipeline_should_extract_archive_rows_from_real_go_html() -> Result<(), Box<dyn Error>> {
        let Ok(page) = std::fs::read_to_string("/tmp/go-dl.html") else {
            return Ok(());
        };
        let lua = Lua::new();
        let current_dir = std::env::current_dir()?;
        set_package_paths(&lua, &current_dir)?;
        register_builtin_modules(&lua, "vs-test/0.1.0", None)?;
        lua.globals().set("page", page)?;

        lua.load(
            r#"
            local html = require("html")
            local doc = html.parse(page)
            local listDoc = doc:find("div#archive")
            debug_info = {}
            result = {}
            listDoc:find(".toggle"):each(function(i, selection)
              local versionStr = selection:attr("id")
              if debug_info.first_toggle_id == nil then
                debug_info.first_toggle_id = versionStr
              end
              if versionStr ~= nil then
                selection:find("table.downloadtable tbody tr"):each(function(ti, ts)
                  local td = ts:find("td")
                  local filename = td:eq(0):text()
                  local kind = td:eq(1):text()
                  local os = td:eq(2):text()
                  local arch = td:eq(3):text()
                  if debug_info.first_kind == nil then
                    debug_info.first_kind = kind
                    debug_info.first_os = os
                    debug_info.first_arch = arch
                    debug_info.first_file = filename
                  end
                  if kind == "Archive" and os == "macOS" and arch == "ARM64" then
                    table.insert(result, {
                      version = string.sub(versionStr, 3),
                      file = filename,
                    })
                  end
                end)
              end
            end)
            "#,
        )
        .set_name("real_go_html_test")
        .exec()?;

        let result: mlua::Table = lua.globals().get("result")?;
        let first: Option<mlua::Table> = result.get(1)?;
        assert!(first.is_some());
        Ok(())
    }
}
