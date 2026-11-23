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
  readonly icon: LucideIcon;
}

export const navigationItems: readonly NavigationItem[] = [
  {
    path: "/",
    label: "Home",
    title: "Home",
    description: "Discover projects and inspect saved FL Studio data.",
    icon: House,
  },
  {
    path: "/library",
    label: "Library",
    title: "Library",
    description: "Browse your FL Studio projects and inspect saved details.",
    icon: LibraryBig,
  },
  {
    path: "/board",
    label: "Board",
    title: "Board",
    description: "A future view of projects moving through production.",
    icon: Columns3,
  },
  {
    path: "/preferences",
    label: "Preferences",
    title: "Preferences",
    description:
      "Choose where your local workspace opens and manage scan roots.",
    icon: SlidersHorizontal,
  },
];

const notFoundRoute: NavigationItem = {
  path: "/",
  label: "Home",
  title: "Page not found",
  description: "This location is not part of the Fruitboard shell.",
  icon: House,
};

export function getNavigationItem(pathname: string): NavigationItem {
  const normalizedPath = pathname.toLowerCase().replace(/\/+$/, "") || "/";

  return (
    navigationItems.find((item) => item.path === normalizedPath) ??
    notFoundRoute
  );
}
