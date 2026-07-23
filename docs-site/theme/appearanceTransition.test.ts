import { expect, test } from "bun:test";

import { toggleAppearanceTheme } from "./appearanceTransition";

test("uses the switch center as the appearance animation origin", async () => {
  let animatedKeyframes:
    | {
        clipPath?: string[];
      }
    | undefined;

  const result = toggleAppearanceTheme({
    currentTheme: "light",
    animationEnabled: true,
    triggerBounds: {
      left: 100,
      top: 40,
      width: 24,
      height: 24,
    },
    setTheme: () => {},
    environment: {
      innerWidth: 1440,
      innerHeight: 900,
      prefersReducedMotion: false,
      startViewTransition: (update) => {
        void update();
        return { ready: Promise.resolve() };
      },
      animateRoot: (keyframes) => {
        animatedKeyframes = keyframes;
        return { finished: Promise.resolve() };
      },
      appendStyle: () => ({
        remove: () => {},
      }),
    },
  });

  await result.animationFinished;

  expect(animatedKeyframes?.clipPath).toEqual([
    "circle(0px at 112px 52px)",
    expect.stringContaining(" at 112px 52px)"),
  ]);
});

test("reveals the target theme layer during the appearance animation", async () => {
  let animationOptions:
    | {
        pseudoElement: string;
      }
    | undefined;

  const result = toggleAppearanceTheme({
    currentTheme: "light",
    animationEnabled: true,
    triggerBounds: {
      left: 12,
      top: 18,
      width: 24,
      height: 24,
    },
    setTheme: () => {},
    environment: {
      innerWidth: 1280,
      innerHeight: 720,
      prefersReducedMotion: false,
      startViewTransition: (update) => {
        void update();
        return { ready: Promise.resolve() };
      },
      animateRoot: (_keyframes, options) => {
        animationOptions = options;
        return { finished: Promise.resolve() };
      },
      appendStyle: () => ({
        remove: () => {},
      }),
    },
  });

  await result.animationFinished;

  expect(animationOptions?.pseudoElement).toBe(
    "::view-transition-new(root)",
  );
});
