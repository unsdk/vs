//! Interactive terminal prompts and selectors used by the CLI.

use std::fmt::Write as _;
use std::io::{IsTerminal, stdin, stdout};

use anyhow::Result;
use dialoguer::{
    Confirm,
    console::{Key, Term},
    theme::{ColorfulTheme, Theme},
};
use fuzzy_matcher::{FuzzyMatcher, skim::SkimMatcherV2};
use vs_core::App;
use vs_plugin_api::AvailableVersion;

use crate::output::version_label;

const LIST_PAGE_SIZE: usize = 20;
const NO_MATCHES_LABEL: &str = "  No matches";

struct PageRenderState<'a> {
    prompt: &'a str,
    search_term: &'a str,
    cursor: usize,
    filtered: &'a [(usize, String)],
    selection: Option<usize>,
    page_start: usize,
    page_size: usize,
}

/// Returns `true` when interactive prompts are safe to show.
pub fn should_use_interactive_tui() -> bool {
    // Skip prompts in CI even if the streams look interactive so scripted runs stay deterministic.
    stdin().is_terminal() && stdout().is_terminal() && std::env::var_os("CI").is_none()
}

/// Runs the interactive `search` flow for a plugin.
///
/// # Errors
///
/// Returns an error if the selector cannot be rendered or the chosen version fails to install.
pub fn run_search_tui(app: &App, plugin: &str, versions: &[AvailableVersion]) -> Result<i32> {
    if versions.is_empty() {
        return Ok(0);
    }

    let selection = select_version(plugin, versions)?;

    if let Some(index) = selection {
        let selected = &versions[index];
        let installed = app.install_plugin_version(plugin, Some(&selected.version))?;
        println!(
            "Install {}@{} success! ",
            installed.plugin, installed.version
        );
        println!(
            "Please use `vs use {}@{}` to use it.",
            installed.plugin, installed.version
        );
    }

    Ok(0)
}

/// Asks whether the user wants to pick a version interactively.
pub fn prompt_for_version_selection(plugin: &str) -> Result<bool> {
    Confirm::with_theme(&ColorfulTheme::default())
        .with_prompt(format!(
            "No {plugin} version provided, do you want to select a version to install?"
        ))
        .default(false)
        .interact()
        .map_err(Into::into)
}

/// Asks whether a missing plugin should be added before continuing.
pub fn prompt_for_plugin_addition(plugin: &str) -> Result<bool> {
    Confirm::with_theme(&ColorfulTheme::default())
        .with_prompt(format!(
            "Plugin {plugin} is not added yet. Do you want to add it now?"
        ))
        .default(false)
        .interact()
        .map_err(Into::into)
}

/// Asks whether every configured plugin and SDK should be installed.
pub fn prompt_for_install_all() -> Result<bool> {
    Confirm::with_theme(&ColorfulTheme::default())
        .with_prompt("Do you want to install these plugins and SDKs?")
        .default(true)
        .interact()
        .map_err(Into::into)
}

/// Asks for confirmation before removing a plugin and its installed SDKs.
pub fn prompt_for_remove_confirmation() -> Result<bool> {
    Confirm::with_theme(&ColorfulTheme::default())
        .with_prompt("Please confirm")
        .default(false)
        .interact()
        .map_err(Into::into)
}

/// Asks whether to upgrade the `vs` binary to the discovered latest version.
pub fn prompt_for_upgrade(current_version: &str, latest_version: &str) -> Result<bool> {
    Confirm::with_theme(&ColorfulTheme::default())
        .with_prompt(format!(
            "Upgrade vs from {current_version} to {latest_version}?"
        ))
        .default(true)
        .interact()
        .map_err(Into::into)
}

/// Presents available versions for a plugin and returns the selected index.
pub fn select_version(plugin: &str, versions: &[AvailableVersion]) -> Result<Option<usize>> {
    let labels = versions.iter().map(version_label).collect::<Vec<_>>();
    pageable_fuzzy_select(&format!("Select a version of {plugin} to install"), &labels)
}

/// Presents installed versions for a plugin and returns the selected index.
pub fn select_installed_version(plugin: &str, versions: &[String]) -> Result<Option<usize>> {
    pageable_fuzzy_select(&format!("Select a version of {plugin} to use"), versions)
}

fn pageable_fuzzy_select(prompt: &str, items: &[String]) -> Result<Option<usize>> {
    if items.is_empty() {
        return Ok(None);
    }

    let term = Term::stderr();
    let theme = ColorfulTheme::default();
    let matcher = SkimMatcherV2::default();
    let mut search_term = String::new();
    let mut cursor = 0usize;
    let mut selection = Some(0usize);
    let mut page_start = 0usize;
    let mut rendered_lines = 0usize;

    term.hide_cursor()?;
    let _cursor_guard = CursorGuard { term: &term };

    loop {
        if rendered_lines > 0 {
            term.clear_last_lines(rendered_lines)?;
        }

        let page_size = page_size_for_rows(term.size().0 as usize);
        let filtered = filtered_items(items, &search_term, &matcher);
        normalize_state(&filtered, page_size, &mut selection, &mut page_start);

        render_page(
            &term,
            &theme,
            PageRenderState {
                prompt,
                search_term: &search_term,
                cursor,
                filtered: &filtered,
                selection,
                page_start,
                page_size,
            },
        )?;
        rendered_lines = rendered_line_count(&filtered, page_start, page_size);
        term.flush()?;

        let byte_indices = search_term_byte_indices(&search_term);

        match term.read_key()? {
            Key::Escape => {
                term.clear_last_lines(rendered_lines)?;
                return Ok(None);
            }
            Key::ArrowUp | Key::BackTab if !filtered.is_empty() => {
                if let Some(selected) = selection {
                    let len = filtered.len();
                    let next = if selected == 0 { len - 1 } else { selected - 1 };
                    selection = Some(next);
                    if next == len - 1 && selected == 0 {
                        page_start = last_page_start(len, page_size);
                    } else if next < page_start {
                        page_start = next;
                    }
                }
            }
            Key::ArrowDown | Key::Tab if !filtered.is_empty() => {
                if let Some(selected) = selection {
                    let len = filtered.len();
                    let next = (selected + 1) % len;
                    selection = Some(next);
                    if next == 0 {
                        page_start = 0;
                    } else if next >= page_start + page_size {
                        page_start = next + 1 - page_size;
                    }
                }
            }
            Key::ArrowLeft | Key::PageUp if !filtered.is_empty() => {
                if let Some(selected) = selection {
                    let (next_selection, next_page_start) =
                        move_to_previous_page(selected, filtered.len(), page_size);
                    selection = Some(next_selection);
                    page_start = next_page_start;
                }
            }
            Key::ArrowRight | Key::PageDown if !filtered.is_empty() => {
                if let Some(selected) = selection {
                    let (next_selection, next_page_start) =
                        move_to_next_page(selected, filtered.len(), page_size);
                    selection = Some(next_selection);
                    page_start = next_page_start;
                }
            }
            Key::Home => {
                cursor = 0;
            }
            Key::End => {
                cursor = byte_indices.len().saturating_sub(1);
            }
            Key::Enter if !filtered.is_empty() => {
                if let Some(selected) = selection {
                    let original_index = filtered[selected].0;
                    term.clear_last_lines(rendered_lines)?;
                    term.write_line(&format_input_prompt_selection(
                        &theme,
                        prompt,
                        &filtered[selected].1,
                    )?)?;
                    return Ok(Some(original_index));
                }
            }
            Key::Backspace if cursor > 0 => {
                cursor -= 1;
                let start = byte_indices[cursor];
                search_term.remove(start);
                selection = Some(0);
                page_start = 0;
            }
            Key::Del if cursor < byte_indices.len().saturating_sub(1) => {
                let start = byte_indices[cursor];
                search_term.remove(start);
                selection = Some(0);
                page_start = 0;
            }
            Key::Char(chr) if !chr.is_ascii_control() => {
                let insert_at = byte_indices[cursor];
                search_term.insert(insert_at, chr);
                cursor += 1;
                selection = Some(0);
                page_start = 0;
            }
            _ => {}
        }
    }
}

fn render_page(term: &Term, theme: &dyn Theme, state: PageRenderState<'_>) -> Result<()> {
    let prompt_line = format_fuzzy_prompt(
        theme,
        state.prompt,
        state.search_term,
        state.cursor,
        state.filtered.len(),
        state.page_start,
        state.page_size,
    )?;
    term.write_line(&prompt_line)?;

    if state.filtered.is_empty() {
        term.write_line(NO_MATCHES_LABEL)?;
        return Ok(());
    }

    let matcher = SkimMatcherV2::default();
    let highlight_matches = true;
    let end = (state.page_start + state.page_size).min(state.filtered.len());

    for (visible_idx, (_, item)) in state.filtered[state.page_start..end].iter().enumerate() {
        let active = state.selection == Some(state.page_start + visible_idx);
        let line = format_fuzzy_item(
            theme,
            item,
            active,
            highlight_matches,
            &matcher,
            state.search_term,
        )?;
        term.write_line(&line)?;
    }

    Ok(())
}

fn format_fuzzy_prompt(
    theme: &dyn Theme,
    prompt: &str,
    search_term: &str,
    cursor: usize,
    total_items: usize,
    page_start: usize,
    page_size: usize,
) -> Result<String> {
    let mut line = String::new();
    theme
        .format_fuzzy_select_prompt(
            &mut line,
            prompt,
            search_term,
            byte_offset(search_term, cursor),
        )
        .map_err(|_| std::io::Error::other("failed to render fuzzy prompt"))?;

    let total_pages = total_items.div_ceil(page_size.max(1));
    if total_pages > 1 {
        let current_page = (page_start / page_size) + 1;
        write!(&mut line, " [Page {current_page}/{total_pages}]")?;
    }

    Ok(line)
}

fn format_fuzzy_item(
    theme: &dyn Theme,
    item: &str,
    active: bool,
    highlight_matches: bool,
    matcher: &SkimMatcherV2,
    search_term: &str,
) -> Result<String> {
    let mut line = String::new();
    theme
        .format_fuzzy_select_prompt_item(
            &mut line,
            item,
            active,
            highlight_matches,
            matcher,
            search_term,
        )
        .map_err(|_| std::io::Error::other("failed to render fuzzy item"))?;
    Ok(line)
}

fn format_input_prompt_selection(
    theme: &dyn Theme,
    prompt: &str,
    selection: &str,
) -> Result<String> {
    let mut line = String::new();
    theme
        .format_input_prompt_selection(&mut line, prompt, selection)
        .map_err(|_| std::io::Error::other("failed to render selection"))?;
    Ok(line)
}

fn filtered_items(
    items: &[String],
    search_term: &str,
    matcher: &SkimMatcherV2,
) -> Vec<(usize, String)> {
    if search_term.is_empty() {
        return items
            .iter()
            .enumerate()
            .map(|(index, item)| (index, item.clone()))
            .collect();
    }

    let mut filtered = items
        .iter()
        .enumerate()
        .filter_map(|(index, item)| {
            matcher
                .fuzzy_match(item, search_term)
                .map(|score| (index, item, score))
        })
        .collect::<Vec<_>>();
    filtered
        .sort_unstable_by(|(_, _, left_score), (_, _, right_score)| right_score.cmp(left_score));
    filtered
        .into_iter()
        .map(|(index, item, _)| (index, item.clone()))
        .collect()
}

fn normalize_state(
    filtered: &[(usize, String)],
    page_size: usize,
    selection: &mut Option<usize>,
    page_start: &mut usize,
) {
    if filtered.is_empty() {
        *selection = None;
        *page_start = 0;
        return;
    }

    let max_index = filtered.len() - 1;
    let selected = selection.unwrap_or(0).min(max_index);
    let max_page_start = last_page_start(filtered.len(), page_size);

    *selection = Some(selected);
    *page_start = (*page_start).min(max_page_start);

    if selected < *page_start {
        *page_start = selected;
    } else if selected >= *page_start + page_size {
        *page_start = selected + 1 - page_size;
    }
}

fn page_size_for_rows(rows: usize) -> usize {
    (rows.max(3) - 2).clamp(1, LIST_PAGE_SIZE)
}

fn last_page_start(total_items: usize, page_size: usize) -> usize {
    total_items.saturating_sub(1) / page_size.max(1) * page_size.max(1)
}

fn move_to_previous_page(selection: usize, total_items: usize, page_size: usize) -> (usize, usize) {
    let current_page_start = selection / page_size * page_size;
    let next_page_start = current_page_start.saturating_sub(page_size);
    let offset = selection - current_page_start;
    let next_page_end = (next_page_start + page_size).min(total_items);
    let next_selection = (next_page_start + offset).min(next_page_end.saturating_sub(1));
    (next_selection, next_page_start)
}

fn move_to_next_page(selection: usize, total_items: usize, page_size: usize) -> (usize, usize) {
    let current_page_start = selection / page_size * page_size;
    let next_page_start =
        (current_page_start + page_size).min(last_page_start(total_items, page_size));
    let offset = selection - current_page_start;
    let next_page_end = (next_page_start + page_size).min(total_items);
    let next_selection = (next_page_start + offset).min(next_page_end.saturating_sub(1));
    (next_selection, next_page_start)
}

fn rendered_line_count(filtered: &[(usize, String)], page_start: usize, page_size: usize) -> usize {
    if filtered.is_empty() {
        2
    } else {
        1 + (filtered.len() - page_start).min(page_size)
    }
}

fn search_term_byte_indices(search_term: &str) -> Vec<usize> {
    let mut byte_indices = search_term
        .char_indices()
        .map(|(index, _)| index)
        .collect::<Vec<_>>();
    byte_indices.push(search_term.len());
    byte_indices
}

fn byte_offset(search_term: &str, cursor: usize) -> usize {
    search_term_byte_indices(search_term)
        .get(cursor)
        .copied()
        .unwrap_or(search_term.len())
}

struct CursorGuard<'a> {
    term: &'a Term,
}

impl Drop for CursorGuard<'_> {
    fn drop(&mut self) {
        let _ = self.term.show_cursor();
    }
}

#[cfg(test)]
mod tests {
    use super::{last_page_start, move_to_next_page, move_to_previous_page, page_size_for_rows};

    #[test]
    fn page_size_should_cap_at_twenty_items() {
        assert_eq!(page_size_for_rows(24), 20);
        assert_eq!(page_size_for_rows(10), 8);
    }

    #[test]
    fn previous_page_should_preserve_offset_when_possible() {
        assert_eq!(move_to_previous_page(27, 55, 20), (7, 0));
        assert_eq!(move_to_previous_page(52, 55, 20), (32, 20));
    }

    #[test]
    fn next_page_should_clamp_on_last_page() {
        assert_eq!(move_to_next_page(7, 55, 20), (27, 20));
        assert_eq!(move_to_next_page(32, 55, 20), (52, 40));
        assert_eq!(move_to_next_page(52, 55, 20), (52, 40));
    }

    #[test]
    fn last_page_start_should_align_to_page_boundary() {
        assert_eq!(last_page_start(55, 20), 40);
        assert_eq!(last_page_start(20, 20), 0);
        assert_eq!(last_page_start(1, 20), 0);
    }
}
