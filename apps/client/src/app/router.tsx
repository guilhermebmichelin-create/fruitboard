import {
  createHashRouter,
  createMemoryRouter,
  type RouteObject,
} from "react-router";
import type { PlatformPort } from "../platform/contracts";
import type {
  LibraryRenderContext,
  LibraryScanAdapter,
} from "../library/contracts";
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
  libraryRenderContext: LibraryRenderContext = "native",
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
          element: (
            <LibraryPage
              adapter={libraryAdapter}
              renderContext={libraryRenderContext}
            />
          ),
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
  libraryRenderContext: LibraryRenderContext = "native",
) {
  return createHashRouter(
    createRoutes(platform, libraryAdapter, libraryRenderContext),
  );
}

export function createFruitboardMemoryRouter(
  platform: PlatformPort,
  initialEntries: string[] = ["/"],
  libraryAdapter?: LibraryScanAdapter,
  libraryRenderContext: LibraryRenderContext = "native",
) {
  return createMemoryRouter(
    createRoutes(platform, libraryAdapter, libraryRenderContext),
    { initialEntries },
  );
}
