import { expect, test } from "bun:test";

import {
  getViewTransitionBlockCss,
  toggleAppearanceTheme,
} from "./appearanceTransition";

function deferredPromise() {
  let resolve: (() => void) | undefined;
  const promise = new Promise<void>((resolvePromise) => {
    resolve = resolvePromise;
  });

  return {
    promise,
    resolve: () => resolve?.(),
  };
}

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

  expect(animatedKeyframes?.clipPath).toHaveLength(2);
  expect(animatedKeyframes?.clipPath?.[0]).toEqual(
    expect.stringContaining(" at 112px 52px)"),
  );
  expect(animatedKeyframes?.clipPath?.[1]).toEqual(
    expect.stringContaining(" at 112px 52px)"),
  );
  expect(animatedKeyframes?.clipPath).toContain("circle(0px at 112px 52px)");
});

test("uses the click position as the appearance animation origin when provided", async () => {
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
    triggerPoint: {
      x: 118,
      y: 47,
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

  expect(animatedKeyframes?.clipPath).toHaveLength(2);
  expect(animatedKeyframes?.clipPath?.[0]).toEqual(
    expect.stringContaining(" at 118px 47px)"),
  );
  expect(animatedKeyframes?.clipPath?.[1]).toEqual(
    expect.stringContaining(" at 118px 47px)"),
  );
  expect(animatedKeyframes?.clipPath).not.toContain("circle(0px at 112px 52px)");
});

test("keeps appearance animation coordinates in CSS pixels on high-density displays", async () => {
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
      devicePixelRatio: 2,
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

  expect(animatedKeyframes?.clipPath?.[0]).toEqual(
    expect.stringContaining(" at 112px 52px)"),
  );
  expect(animatedKeyframes?.clipPath?.[1]).toEqual(
    expect.stringContaining(" at 112px 52px)"),
  );
  expect(animatedKeyframes?.clipPath?.[0]).not.toContain("224px 104px");
});

test("raises the revealed layer above the background layer", () => {
  const darkCss = getViewTransitionBlockCss(true);
  const lightCss = getViewTransitionBlockCss(false);

  expect(darkCss).toContain(".rp-nav *::before");
  expect(darkCss).toContain(".docs-button *::before");
  expect(darkCss).toContain("transition: none !important;");
  expect(darkCss).not.toContain("html,");
  expect(darkCss).not.toContain("body *::before");
  expect(darkCss).toContain("::view-transition-new(root)");
  expect(darkCss).toContain("z-index: 9999 !important;");
  expect(darkCss).toContain("::view-transition-old(root)");
  expect(darkCss).toContain("z-index: 1 !important;");
  expect(lightCss).toContain("::view-transition-old(root)");
  expect(lightCss).toContain("z-index: 9999 !important;");
  expect(lightCss).toContain("::view-transition-new(root)");
  expect(lightCss).toContain("z-index: 1 !important;");
});

test("reveals the target dark layer from the switch center", async () => {
  let animationOptions:
    | {
        pseudoElement: string;
      }
    | undefined;
  let animatedKeyframes:
    | {
        clipPath?: string[];
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
      animateRoot: (keyframes, options) => {
        animatedKeyframes = keyframes;
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
  expect(animatedKeyframes?.clipPath?.[0]).toBe("circle(0px at 24px 30px)");
  expect(animatedKeyframes?.clipPath?.[1]).not.toBe("circle(0px at 24px 30px)");
});

test("shrinks the current dark layer back into the switch center", async () => {
  let animationOptions:
    | {
        fill: string;
        pseudoElement: string;
      }
    | undefined;
  let animatedKeyframes:
    | {
        clipPath?: string[];
      }
    | undefined;

  const result = toggleAppearanceTheme({
    currentTheme: "dark",
    animationEnabled: true,
    triggerBounds: {
      left: 24,
      top: 12,
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
      animateRoot: (keyframes, options) => {
        animatedKeyframes = keyframes;
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
    "::view-transition-old(root)",
  );
  expect(animationOptions?.fill).toBe("forwards");
  expect(animatedKeyframes?.clipPath?.[0]).not.toBe(
    "circle(0px at 36px 24px)",
  );
  expect(animatedKeyframes?.clipPath?.[1]).toBe("circle(0px at 36px 24px)");
});

test("keeps transition overrides until the browser view transition fully finishes", async () => {
  let cancelCalls = 0;
  let removeCalls = 0;
  const rootAnimation = deferredPromise();
  const viewTransition = deferredPromise();
  const animationHandle = {
    cancel: () => {
      cancelCalls += 1;
    },
    finished: rootAnimation.promise,
  };

  const result = toggleAppearanceTheme({
    currentTheme: "light",
    animationEnabled: true,
    triggerBounds: {
      left: 24,
      top: 12,
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
        return {
          ready: Promise.resolve(),
          finished: viewTransition.promise,
        };
      },
      animateRoot: () => animationHandle,
      appendStyle: () => ({
        remove: () => {
          removeCalls += 1;
        },
      }),
    },
  });

  rootAnimation.resolve();
  await Promise.resolve();
  expect(cancelCalls).toBe(0);
  expect(removeCalls).toBe(0);

  viewTransition.resolve();
  await result.animationFinished;
  expect(cancelCalls).toBe(1);
  expect(removeCalls).toBe(1);
});
