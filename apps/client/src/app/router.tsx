import {
  createHashRouter,
  createMemoryRouter,
  type RouteObject,
} from "react-router";
import type { PlatformPort } from "../platform/contracts";
import type { LibraryScanAdapter } from "../library/contracts";
import { SafeApplicationError } from "./AppErrorBoundary";
import { FruitboardApp } from "./FruitboardApp";
import {
  BoardPage,
  HomePage,
  LibraryPage,
  NotFoundPage,
  PreferencesPage,
} from "./pages";

export function createRoutes(
  platform: PlatformPort,
  libraryAdapter?: LibraryScanAdapter,
): RouteObject[] {
  return [
    {
      path: "/",
      element: <FruitboardApp platform={platform} />,
      errorElement: <SafeApplicationError />,
      children: [
        { index: true, element: <HomePage /> },
        {
          path: "library",
          element: <LibraryPage adapter={libraryAdapter} />,
        },
        { path: "board", element: <BoardPage /> },
        {
          path: "preferences",
          element: <PreferencesPage platform={platform} />,
        },
        { path: "*", element: <NotFoundPage /> },
      ],
    },
  ];
}

export function createFruitboardHashRouter(
  platform: PlatformPort,
  libraryAdapter?: LibraryScanAdapter,
) {
  return createHashRouter(createRoutes(platform, libraryAdapter));
}

export function createFruitboardMemoryRouter(
  platform: PlatformPort,
  initialEntries: string[] = ["/"],
  libraryAdapter?: LibraryScanAdapter,
) {
  return createMemoryRouter(createRoutes(platform, libraryAdapter), {
    initialEntries,
  });
}
