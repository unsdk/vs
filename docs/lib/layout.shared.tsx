import type { BaseLayoutProps } from 'fumadocs-ui/layouts/shared';
import { appName, docsRoute, repoUrl } from './shared';

export function baseOptions(): BaseLayoutProps {
  return {
    nav: {
      title: appName,
      url: '/',
    },
    links: [
      {
        text: 'Docs',
        url: docsRoute,
        active: 'nested-url',
      },
      {
        text: 'Guides',
        url: '/docs/guides',
        active: 'nested-url',
      },
      {
        text: 'Project',
        url: '/docs/project',
        active: 'nested-url',
      },
    ],
    githubUrl: repoUrl,
  };
}
