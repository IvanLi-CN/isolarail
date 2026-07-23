export type ThemeMode = "dark" | "light";

export type TriggerBounds = {
  left: number;
  top: number;
  width: number;
  height: number;
};

type AnimationKeyframes = {
  clipPath?: string[];
};

type AnimationOptions = {
  duration: number;
  easing: string;
  pseudoElement: string;
  id: string;
};

type AnimationHandle = {
  finished: Promise<unknown>;
};

type ViewTransition = {
  ready: Promise<unknown>;
};

export type AppearanceTransitionEnvironment = {
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
  setTheme: (theme: ThemeMode) => void;
  onToggle?: () => void;
  environment: AppearanceTransitionEnvironment;
  flushSync?: (callback: () => void) => void;
};

type ToggleAppearanceThemeResult = {
  nextTheme: ThemeMode;
  animationFinished: Promise<void>;
};

const VIEW_TRANSITION_BLOCK_CSS = `
  .rspress-doc {
    view-transition-name: none !important;
  }
`;

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

  const { x, y } = getAppearanceAnimationOrigin(triggerBounds);
  const endRadius = Math.hypot(
    Math.max(x, environment.innerWidth - x + 200),
    Math.max(y, environment.innerHeight - y + 200),
  );
  const cleanup = environment.appendStyle(VIEW_TRANSITION_BLOCK_CSS);
  const applyTheme = flushSync ?? ((callback: () => void) => callback());
  const transition = environment.startViewTransition(() => {
    applyTheme(() => {
      setTheme(nextTheme);
      onToggle?.();
    });
  });

  const animationFinished = transition.ready
    .then(() =>
      environment.animateRoot(
        {
          clipPath: [
            `circle(0px at ${x}px ${y}px)`,
            `circle(${endRadius}px at ${x}px ${y}px)`,
          ],
        },
        {
          duration: 400,
          easing: "ease-in",
          pseudoElement: "::view-transition-new(root)",
          id: "",
        },
      ).finished,
    )
    .finally(() => {
      cleanup.remove();
    })
    .then(() => undefined);

  return {
    nextTheme,
    animationFinished,
  };
}
