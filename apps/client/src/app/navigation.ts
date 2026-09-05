import {
  Columns3,
  House,
  LibraryBig,
  SlidersHorizontal,
  type LucideIcon,
} from "lucide-react";

export interface NavigationItem {
  readonly path: string;
  readonly label: string;
  readonly title: string;
  readonly description: string;
  readonly actionLabel: string;
  readonly actionPath: string;
  readonly icon: LucideIcon;
}

export const navigationItems: readonly NavigationItem[] = [
  {
    path: "/",
    label: "Home",
    title: "Home",
    description: "Your local production workspace at a glance.",
    actionLabel: "View library",
    actionPath: "/library",
    icon: House,
  },
  {
    path: "/library",
    label: "Library",
    title: "Library",
    description: "FL Studio projects will be collected here.",
    actionLabel: "Return home",
    actionPath: "/",
    icon: LibraryBig,
  },
  {
    path: "/board",
    label: "Board",
    title: "Board",
    description: "A future view of projects moving through production.",
    actionLabel: "View library",
    actionPath: "/library",
    icon: Columns3,
  },
  {
    path: "/preferences",
    label: "Preferences",
    title: "Preferences",
    description: "The interface baseline for this local workspace.",
    actionLabel: "Return home",
    actionPath: "/",
    icon: SlidersHorizontal,
  },
];

const notFoundRoute: NavigationItem = {
  path: "/",
  label: "Home",
  title: "Page not found",
  description: "This location is not part of the Fruitboard shell.",
  actionLabel: "Return home",
  actionPath: "/",
  icon: House,
};

export function getNavigationItem(pathname: string): NavigationItem {
  const normalizedPath = pathname.toLowerCase().replace(/\/+$/, "") || "/";

  return (
    navigationItems.find((item) => item.path === normalizedPath) ??
    notFoundRoute
  );
}
