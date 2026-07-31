import { Easing, interpolate } from "remotion";

export const easeOut = Easing.out(Easing.cubic);
export const easeSharp = Easing.out(Easing.quad);

export function progress(frame: number, start: number, end: number) {
  return interpolate(frame, [start, end], [0, 1], {
    extrapolateLeft: "clamp",
    extrapolateRight: "clamp",
    easing: easeOut,
  });
}

export function fadeRange(
  frame: number,
  start: number,
  inEnd: number,
  outStart: number,
  end: number,
) {
  const fadeIn = interpolate(frame, [start, inEnd], [0, 1], {
    extrapolateLeft: "clamp",
    extrapolateRight: "clamp",
    easing: easeOut,
  });
  const fadeOut = interpolate(frame, [outStart, end], [1, 0], {
    extrapolateLeft: "clamp",
    extrapolateRight: "clamp",
    easing: easeSharp,
  });
  return Math.min(fadeIn, fadeOut);
}

export function slideY(frame: number, start: number, end: number, amount = 70) {
  return interpolate(frame, [start, end], [amount, 0], {
    extrapolateLeft: "clamp",
    extrapolateRight: "clamp",
    easing: easeOut,
  });
}
