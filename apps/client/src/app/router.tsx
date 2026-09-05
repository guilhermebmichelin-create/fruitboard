import {
  createHashRouter,
  createMemoryRouter,
  type RouteObject,
} from "react-router";
import type { PlatformPort } from "../platform/contracts";
import { SafeApplicationError } from "./AppErrorBoundary";
import { FruitboardApp } from "./FruitboardApp";
import {
  BoardPage,
  HomePage,
  LibraryPage,
  NotFoundPage,
  PreferencesPage,
} from "./pages";

export function createRoutes(platform: PlatformPort): RouteObject[] {
  return [
    {
      path: "/",
      element: <FruitboardApp platform={platform} />,
      errorElement: <SafeApplicationError />,
      children: [
        { index: true, element: <HomePage /> },
        { path: "library", element: <LibraryPage /> },
        { path: "board", element: <BoardPage /> },
        { path: "preferences", element: <PreferencesPage /> },
        { path: "*", element: <NotFoundPage /> },
      ],
    },
  ];
}

export function createFruitboardHashRouter(platform: PlatformPort) {
  return createHashRouter(createRoutes(platform));
}

export function createFruitboardMemoryRouter(
  platform: PlatformPort,
  initialEntries: string[] = ["/"],
) {
  return createMemoryRouter(createRoutes(platform), { initialEntries });
}
