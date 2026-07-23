export type ThemeMode = "dark" | "light";

const TRANSITION_FREEZE_SELECTORS = [
  ".rp-nav",
  ".rp-nav *",
  ".rp-nav *::before",
  ".rp-nav *::after",
  ".docs-button",
  ".docs-button *",
  ".docs-button *::before",
  ".docs-button *::after",
  ".docs-inline-links a",
  ".docs-inline-links a::before",
  ".docs-inline-links a::after",
  ".docs-switchboard-route",
  ".docs-switchboard-route *",
  ".docs-switchboard-route *::before",
  ".docs-switchboard-route *::after",
  ".docs-manual-stop",
  ".docs-manual-stop *",
  ".docs-manual-stop *::before",
  ".docs-manual-stop *::after",
  ".docs-manual-links a",
  ".docs-manual-links a::before",
  ".docs-manual-links a::after",
].join(",\n    ");

export type TriggerBounds = {
  left: number;
  top: number;
  width: number;
  height: number;
};

export type TriggerPoint = {
  x: number;
  y: number;
};

type AnimationKeyframes = {
  clipPath?: string[];
};

type AnimationOptions = {
  duration: number;
  easing: string;
  fill: "forwards";
  pseudoElement: string;
  id: string;
};

type AnimationHandle = {
  finished: Promise<unknown>;
};

type ViewTransition = {
  finished?: Promise<unknown>;
  ready: Promise<unknown>;
};

export type AppearanceTransitionEnvironment = {
  devicePixelRatio?: number;
  innerWidth: number;
  innerHeight: number;
  prefersReducedMotion: boolean;
  startViewTransition?: (
    update: () => void | Promise<void>,
  ) => ViewTransition;
  animateRoot: (
    keyframes: AnimationKeyframes,
    options: AnimationOptions,
  ) => AnimationHandle;
  appendStyle: (cssText: string) => {
    remove: () => void;
  };
};

type ToggleAppearanceThemeOptions = {
  currentTheme: ThemeMode;
  animationEnabled: boolean;
  triggerBounds: TriggerBounds;
  triggerPoint?: TriggerPoint;
  setTheme: (theme: ThemeMode) => void;
  onToggle?: () => void;
  environment: AppearanceTransitionEnvironment;
  flushSync?: (callback: () => void) => void;
};

type ToggleAppearanceThemeResult = {
  nextTheme: ThemeMode;
  animationFinished: Promise<void>;
};

export function getViewTransitionBlockCss(revealNewTheme: boolean) {
  const foregroundLayer = revealNewTheme ? "new" : "old";
  const backgroundLayer = revealNewTheme ? "old" : "new";

  return `
    .rspress-doc {
      view-transition-name: none !important;
    }

    ${TRANSITION_FREEZE_SELECTORS} {
      transition: none !important;
    }

    ::view-transition-group(root),
    ::view-transition-image-pair(root),
    ::view-transition-old(root),
    ::view-transition-new(root) {
      animation: none !important;
      mix-blend-mode: normal !important;
    }

    ::view-transition-group(root) {
      isolation: auto;
    }

    ::view-transition-${foregroundLayer}(root) {
      z-index: 9999 !important;
    }

    ::view-transition-${backgroundLayer}(root) {
      z-index: 1 !important;
    }
  `;
}

export function getAppearanceAnimationOrigin(triggerBounds: TriggerBounds) {
  return {
    x: triggerBounds.left + triggerBounds.width / 2,
    y: triggerBounds.top + triggerBounds.height / 2,
  };
}

export function toggleAppearanceTheme(
  options: ToggleAppearanceThemeOptions,
): ToggleAppearanceThemeResult {
  const {
    animationEnabled,
    currentTheme,
    environment,
    flushSync,
    onToggle,
    setTheme,
    triggerBounds,
    triggerPoint,
  } = options;
  const nextTheme: ThemeMode = currentTheme === "dark" ? "light" : "dark";
  const canAnimate =
    animationEnabled &&
    !environment.prefersReducedMotion &&
    typeof environment.startViewTransition === "function";

  if (!canAnimate) {
    setTheme(nextTheme);
    onToggle?.();
    return {
      nextTheme,
      animationFinished: Promise.resolve(),
    };
  }

  const { x, y } =
    triggerPoint ?? getAppearanceAnimationOrigin(triggerBounds);
  const pixelRatio = environment.devicePixelRatio ?? 1;
  const animationX = x * pixelRatio;
  const animationY = y * pixelRatio;
  const animationWidth = environment.innerWidth * pixelRatio;
  const animationHeight = environment.innerHeight * pixelRatio;
  const revealNewTheme = nextTheme === "dark";
  const endRadius = Math.hypot(
    Math.max(animationX, animationWidth - animationX + 200 * pixelRatio),
    Math.max(animationY, animationHeight - animationY + 200 * pixelRatio),
  );
  const cleanup = environment.appendStyle(
    getViewTransitionBlockCss(revealNewTheme),
  );
  const applyTheme = flushSync ?? ((callback: () => void) => callback());
  const transition = environment.startViewTransition(() => {
    applyTheme(() => {
      setTheme(nextTheme);
      onToggle?.();
    });
  });
  const revealClipPath = [
    `circle(0px at ${animationX}px ${animationY}px)`,
    `circle(${endRadius}px at ${animationX}px ${animationY}px)`,
  ];
  const animationClipPath = revealNewTheme
    ? revealClipPath
    : [...revealClipPath].reverse();
  const pseudoElement = revealNewTheme
    ? "::view-transition-new(root)"
    : "::view-transition-old(root)";

  const rootAnimationFinished = transition.ready.then(() =>
    environment.animateRoot(
      {
        clipPath: animationClipPath,
      },
      {
        duration: 400,
        easing: "ease-in",
        fill: "forwards",
        pseudoElement,
        id: "",
      },
    ).finished,
  );
  const transitionFinished = transition.finished ?? rootAnimationFinished;
  const animationFinished = Promise.allSettled([
    rootAnimationFinished,
    transitionFinished,
  ])
    .then(() =>
      undefined,
    )
    .finally(() => {
      cleanup.remove();
    });

  return {
    nextTheme,
    animationFinished,
  };
}
