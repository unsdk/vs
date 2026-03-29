        import Link from 'next/link';
        import {
          ArrowRight,
          Blocks,
          BookOpenText,
          Boxes,
          PlugZap,
          TerminalSquare,
        } from 'lucide-react';

        const featureCards = [
          {
            title: 'Multi-backend runtime management',
            description:
              'Build vs with Lua, WASI, or both backends depending on the plugin model you want to ship.',
            icon: Boxes,
          },
          {
            title: 'Scope-aware activation',
            description:
              'Pin versions at project, session, or global scope and keep the resolution model explicit.',
            icon: Blocks,
          },
          {
            title: 'Plugin-friendly architecture',
            description:
              'Use lightweight Lua hooks or typed native descriptors without changing the CLI surface.',
            icon: PlugZap,
          },
          {
            title: 'Shell-first workflows',
            description:
              'Activate shells, run ad-hoc commands with `vs exec`, or inspect active runtimes directly.',
            icon: TerminalSquare,
          },
        ];

        const docAreas = [
          {
            title: 'Guides',
            description: 'Start from a local checkout, configure a home, add plugins, and activate tools.',
            href: '/docs/guides',
          },
          {
            title: 'Reference',
            description: 'Understand the CLI surface, shell activation, PATH behavior, and compatibility files.',
            href: '/docs/reference',
          },
          {
            title: 'Plugins',
            description: 'Compare Lua plugins and native descriptors, and see how both fit the same lifecycle.',
            href: '/docs/plugins',
          },
          {
            title: 'Project',
            description: 'Explore the workspace layout, crate boundaries, and the implementation model behind the CLI.',
            href: '/docs/project',
          },
        ];

        export default function HomePage() {
          return (
            <div className="mx-auto flex w-full max-w-6xl flex-1 flex-col gap-16 px-6 py-16 sm:px-8 lg:py-24">
              <section className="grid gap-10 lg:grid-cols-[minmax(0,1.1fr)_minmax(320px,0.9fr)] lg:items-center">
                <div className="space-y-6">
                  <div className="inline-flex items-center rounded-full border border-neutral-200 px-3 py-1 text-sm text-neutral-600 dark:border-neutral-800 dark:text-neutral-300">
                    Rust workspace · static docs · GitHub Pages
                  </div>
                  <div className="space-y-4">
                    <h1 className="max-w-3xl text-4xl font-semibold tracking-tight sm:text-5xl lg:text-6xl">
                      Documentation for <span className="text-neutral-500 dark:text-neutral-300">vs</span>
                    </h1>
                    <p className="max-w-2xl text-lg leading-8 text-neutral-600 dark:text-neutral-300">
                      vs is a cross-platform runtime version manager inspired by vfox. This site covers
                      installation flows, CLI behavior, plugin models, and the workspace architecture
                      behind the project.
                    </p>
                  </div>
                  <div className="flex flex-wrap gap-3">
                    <Link
                      href="/docs/guides/quick-start"
                      className="inline-flex items-center gap-2 rounded-full bg-neutral-950 px-5 py-3 text-sm font-medium text-white transition hover:bg-neutral-800 dark:bg-white dark:text-neutral-950 dark:hover:bg-neutral-200"
                    >
                      Get Started
                      <ArrowRight className="size-4" />
                    </Link>
                    <Link
                      href="/docs/reference/cli-commands"
                      className="inline-flex items-center gap-2 rounded-full border border-neutral-300 px-5 py-3 text-sm font-medium text-neutral-700 transition hover:border-neutral-400 hover:text-neutral-950 dark:border-neutral-700 dark:text-neutral-200 dark:hover:border-neutral-500 dark:hover:text-white"
                    >
                      CLI Reference
                      <BookOpenText className="size-4" />
                    </Link>
                  </div>
                  <p className="text-sm text-neutral-500 dark:text-neutral-400">
                    Covers project-scoped version files, session activation, shell hooks, plugin backends,
                    and the workspace crate layout.
                  </p>
                </div>

                <div className="rounded-3xl border border-neutral-200 bg-neutral-50 p-6 shadow-sm dark:border-neutral-800 dark:bg-neutral-900">
                  <div className="mb-4 flex items-center justify-between">
                    <h2 className="text-sm font-semibold uppercase tracking-[0.2em] text-neutral-500 dark:text-neutral-400">
                      Quick start
                    </h2>
                    <span className="rounded-full border border-neutral-200 px-2 py-1 text-xs text-neutral-500 dark:border-neutral-700 dark:text-neutral-400">
                      local workspace
                    </span>
                  </div>
                  <pre className="overflow-x-auto rounded-2xl bg-neutral-950 p-4 text-sm leading-6 text-neutral-100">
                    <code>{`cargo build -p vs-cli
export VS_HOME="$HOME/.vs"
vs config registry.address /absolute/path/to/fixtures/registry/index.json
vs add nodejs
vs install nodejs@20.11.1
vs use nodejs@20.11.1 -g
eval "$(vs activate zsh)"`}</code>
                  </pre>
                  <p className="mt-4 text-sm leading-6 text-neutral-600 dark:text-neutral-300">
                    Start with the local fixture registry while developing, then switch to your preferred
                    registry source or distribution build once you are ready.
                  </p>
                </div>
              </section>

              <section className="space-y-6">
                <div className="space-y-2">
                  <h2 className="text-2xl font-semibold tracking-tight sm:text-3xl">What the docs focus on</h2>
                  <p className="max-w-3xl text-neutral-600 dark:text-neutral-300">
                    The site is organized around the workflows you hit when using or extending vs in a real repository.
                  </p>
                </div>
                <div className="grid gap-4 md:grid-cols-2 xl:grid-cols-4">
                  {featureCards.map(({ title, description, icon: Icon }) => (
                    <div
                      key={title}
                      className="rounded-2xl border border-neutral-200 bg-white p-5 shadow-sm dark:border-neutral-800 dark:bg-neutral-950"
                    >
                      <div className="mb-4 inline-flex rounded-xl border border-neutral-200 p-2 dark:border-neutral-800">
                        <Icon className="size-5" />
                      </div>
                      <h3 className="mb-2 text-lg font-semibold">{title}</h3>
                      <p className="text-sm leading-6 text-neutral-600 dark:text-neutral-300">{description}</p>
                    </div>
                  ))}
                </div>
              </section>

              <section className="space-y-6">
                <div className="space-y-2">
                  <h2 className="text-2xl font-semibold tracking-tight sm:text-3xl">Browse the handbook</h2>
                  <p className="max-w-3xl text-neutral-600 dark:text-neutral-300">
                    Choose the section that matches your current task, from first-run setup to plugin internals.
                  </p>
                </div>
                <div className="grid gap-4 md:grid-cols-2">
                  {docAreas.map((area) => (
                    <Link
                      key={area.title}
                      href={area.href}
                      className="group rounded-2xl border border-neutral-200 bg-white p-6 shadow-sm transition hover:-translate-y-0.5 hover:border-neutral-300 dark:border-neutral-800 dark:bg-neutral-950 dark:hover:border-neutral-700"
                    >
                      <div className="flex items-start justify-between gap-4">
                        <div>
                          <h3 className="text-lg font-semibold">{area.title}</h3>
                          <p className="mt-2 text-sm leading-6 text-neutral-600 dark:text-neutral-300">
                            {area.description}
                          </p>
                        </div>
                        <ArrowRight className="mt-1 size-5 shrink-0 text-neutral-400 transition group-hover:translate-x-1" />
                      </div>
                    </Link>
                  ))}
                </div>
              </section>
            </div>
          );
        }
