import { Layout as BasicLayout } from '@rspress/core/theme-original';

import { DocsNavTitle } from './DocsNavTitle';
export { SwitchAppearance } from "./DocsSwitchAppearance";

export function Layout() {
  return <BasicLayout navTitle={<DocsNavTitle />} />;
}

export * from '@rspress/core/theme-original';
