import {
  AbsoluteFill,
  Sequence,
  interpolate,
  spring,
  useCurrentFrame,
  useVideoConfig,
} from "remotion";
import { CellarMark, DataGridPreview, ProductShot } from "./ProductShots";
import { fadeRange, progress, slideY } from "./motion";

function TitleCard({
  start,
  end,
  children,
  className = "",
}: {
  start: number;
  end: number;
  children: React.ReactNode;
  className?: string;
}) {
  const frame = useCurrentFrame();
  const opacity = fadeRange(frame, start, start + 14, end - 14, end);
  const y = slideY(frame, start, start + 24, 54);
  const scale = interpolate(progress(frame, start, start + 26), [0, 1], [0.94, 1]);

  return (
    <div
      className={`title-card ${className}`}
      style={{
        opacity,
        transform: `translate3d(0, ${y}px, 0) scale(${scale})`,
      }}
    >
      {children}
    </div>
  );
}

function FloatingLabel({
  start,
  end,
  children,
  x,
  y,
}: {
  start: number;
  end: number;
  children: React.ReactNode;
  x: number;
  y: number;
}) {
  const frame = useCurrentFrame();
  const opacity = fadeRange(frame, start, start + 10, end - 10, end);
  const lift = slideY(frame, start, start + 22, 38);

  return (
    <div
      className="floating-label"
      style={{
        left: x,
        top: y,
        opacity,
        transform: `translate3d(0, ${lift}px, 0)`,
      }}
    >
      {children}
    </div>
  );
}

function WindowShot({
  name,
  start,
  end,
  scaleFrom,
  scaleTo,
  xFrom,
  xTo,
  yFrom,
  yTo,
}: {
  name: "main" | "workspace" | "connection" | "cmd";
  start: number;
  end: number;
  scaleFrom: number;
  scaleTo: number;
  xFrom: number;
  xTo: number;
  yFrom: number;
  yTo: number;
}) {
  const frame = useCurrentFrame();
  const opacity = fadeRange(frame, start, start + 16, end - 14, end);
  const p = progress(frame, start, end);
  const scale = interpolate(p, [0, 1], [scaleFrom, scaleTo]);
  const x = interpolate(p, [0, 1], [xFrom, xTo]);
  const y = interpolate(p, [0, 1], [yFrom, yTo]);

  return (
    <div className="shot-frame" style={{ opacity }}>
      <ProductShot
        name={name}
        className="shot-image"
        style={
          {
            "--shot-scale": scale,
            "--shot-x": `${x}px`,
            "--shot-y": `${y}px`,
          } as React.CSSProperties
        }
      />
    </div>
  );
}

function ConnectScene() {
  return (
    <>
      <WindowShot
        name="connection"
        start={78}
        end={188}
        scaleFrom={1.42}
        scaleTo={1.28}
        xFrom={-90}
        xTo={20}
        yFrom={45}
        yTo={-35}
      />
      <FloatingLabel start={98} end={178} x={310} y={990}>
        Connect to Postgres
      </FloatingLabel>
    </>
  );
}

function CommandScene() {
  return (
    <>
      <WindowShot
        name="cmd"
        start={170}
        end={286}
        scaleFrom={1.4}
        scaleTo={1.18}
        xFrom={20}
        xTo={-80}
        yFrom={65}
        yTo={-20}
      />
      <FloatingLabel start={196} end={274} x={1470} y={1000}>
        Search tables, commands, and queries
      </FloatingLabel>
    </>
  );
}

function WorkspaceScene() {
  return (
    <>
      <WindowShot
        name="workspace"
        start={260}
        end={438}
        scaleFrom={1.12}
        scaleTo={1.03}
        xFrom={-90}
        xTo={10}
        yFrom={20}
        yTo={-15}
      />
      <FloatingLabel start={292} end={380} x={330} y={190}>
        Browse schemas without losing the thread
      </FloatingLabel>
    </>
  );
}

function ReviewScene() {
  const frame = useCurrentFrame();
  const opacity = fadeRange(frame, 400, 420, 560, 582);
  const cardIn = spring({
    frame: frame - 410,
    fps: 30,
    config: { damping: 28, stiffness: 120, mass: 0.75 },
  });
  const scale = interpolate(cardIn, [0, 1], [0.9, 1]);

  return (
    <div className="review-scene" style={{ opacity }}>
      <div className="review-card" style={{ transform: `scale(${scale})` }}>
        <div className="review-copy">
          <span className="eyebrow">Review first</span>
          <h2>Edits stay visible until you commit.</h2>
          <p>
            Stage changes in the grid, inspect the diff, then decide what runs.
          </p>
        </div>
        <DataGridPreview />
      </div>
      <div className="commit-strip">
        <span>+1 insert</span>
        <span>2 updates</span>
        <span>1 delete</span>
        <strong>Review and commit</strong>
      </div>
    </div>
  );
}

function PrivacyScene() {
  const frame = useCurrentFrame();
  const opacity = fadeRange(frame, 622, 638, 684, 704);
  const p = progress(frame, 630, 680);
  const scale = interpolate(p, [0, 1], [0.86, 1]);

  return (
    <div className="privacy-scene" style={{ opacity, transform: `scale(${scale})` }}>
      <div className="privacy-line">Local-first</div>
      <div className="privacy-pills">
        <span>No telemetry by default</span>
        <span>Credentials stay in your keychain</span>
        <span>Open source</span>
      </div>
    </div>
  );
}

function RowBurst() {
  const frame = useCurrentFrame();
  const opacity = fadeRange(frame, 555, 570, 620, 636);
  const x = interpolate(progress(frame, 555, 630), [0, 1], [0, -250]);

  return (
    <div className="row-burst" style={{ opacity, transform: `translateX(${x}px)` }}>
      so many rows
    </div>
  );
}

function Outro() {
  const frame = useCurrentFrame();
  const opacity = fadeRange(frame, 676, 692, 716, 720);
  const y = slideY(frame, 676, 704, 58);

  return (
    <div className="outro" style={{ opacity, transform: `translateY(${y}px)` }}>
      <CellarMark className="outro-mark" />
      <div>
        <h1>Cellar</h1>
        <p>Open-source desktop database client.</p>
      </div>
    </div>
  );
}

export function CellarPromo() {
  const frame = useCurrentFrame();
  const { width, height } = useVideoConfig();
  const wash = interpolate(progress(frame, 0, 720), [0, 1], [0, 1]);

  return (
    <AbsoluteFill
      className="stage"
      style={
        {
          "--w": `${width}px`,
          "--h": `${height}px`,
          "--wash": wash,
        } as React.CSSProperties
      }
    >
      <div className="grain" />
      <Sequence from={0}>
        <TitleCard start={-10} end={62}>
          <span>Say hello</span>
        </TitleCard>
      </Sequence>
      <Sequence from={0}>
        <TitleCard start={52} end={116} className="brand-title">
          <CellarMark className="title-mark" />
          <span>to Cellar</span>
        </TitleCard>
      </Sequence>
      <ConnectScene />
      <CommandScene />
      <WorkspaceScene />
      <ReviewScene />
      <PrivacyScene />
      <RowBurst />
      <Outro />
    </AbsoluteFill>
  );
}
