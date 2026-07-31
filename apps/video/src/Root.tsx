import { Composition } from "remotion";
import { CellarPromo } from "./CellarPromo";
import "./styles.css";

export function Root() {
  return (
    <Composition
      id="CellarPromo"
      component={CellarPromo}
      width={2560}
      height={1440}
      fps={30}
      durationInFrames={720}
    />
  );
}
