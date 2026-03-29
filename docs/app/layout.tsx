import { Inter } from 'next/font/google';
import type { Metadata } from 'next';
import { Provider } from '@/components/provider';
import { appDescription, appName, siteUrl } from '@/lib/shared';
import './global.css';

const inter = Inter({
  subsets: ['latin'],
});

export const metadata: Metadata = {
  metadataBase: new URL(siteUrl),
  title: {
    default: `${appName} Documentation`,
    template: `%s | ${appName}`,
  },
  description: appDescription,
  applicationName: appName,
  openGraph: {
    type: 'website',
    siteName: appName,
    url: siteUrl,
    title: `${appName} Documentation`,
    description: appDescription,
  },
  twitter: {
    card: 'summary_large_image',
    title: `${appName} Documentation`,
    description: appDescription,
  },
};

export default function Layout({ children }: LayoutProps<'/'>) {
  return (
    <html lang="en" className={inter.className} suppressHydrationWarning>
      <body className="flex min-h-screen flex-col bg-white text-neutral-950 antialiased dark:bg-neutral-950 dark:text-neutral-50">
        <Provider>{children}</Provider>
      </body>
    </html>
  );
}
