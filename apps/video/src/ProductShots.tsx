import { Img, staticFile } from "remotion";

type ShotName = "main" | "workspace" | "connection" | "cmd";

const shots: Record<ShotName, string> = {
  main: "assets/main.png",
  workspace: "assets/cellar-main.png",
  connection: "assets/connection.png",
  cmd: "assets/cmd.png",
};

export function CellarMark({ className = "" }: { className?: string }) {
  return (
    <Img
      className={className}
      src={staticFile("assets/cellar-mark.svg")}
      alt="Cellar"
    />
  );
}

export function ProductShot({
  name,
  className = "",
  style,
}: {
  name: ShotName;
  className?: string;
  style?: React.CSSProperties;
}) {
  return (
    <Img
      className={className}
      src={staticFile(shots[name])}
      style={style}
      alt=""
    />
  );
}

export function DataGridPreview() {
  const columns = ["id", "country", "orders", "revenue", "refunds", "status"];
  const rows = [
    ["40000000", "DE", "12,480", "$421k", "118", "pending"],
    ["e0000000", "FR", "9,821", "$317k", "74", "clean"],
    ["80000000", "NL", "6,210", "$208k", "39", "clean"],
    ["20000000", "IT", "5,821", "$184k", "51", "edited"],
    ["10000000", "ES", "4,120", "$138k", "22", "clean"],
    ["90000000", "SE", "3,220", "$109k", "18", "new"],
  ];

  return (
    <div className="grid-preview" aria-hidden="true">
      <div className="grid-toolbar">
        <span>public.orders</span>
        <span className="grid-pill">4 pending changes</span>
        <span>1,240 / 219k rows</span>
      </div>
      <div className="grid-table">
        <div className="grid-row grid-head">
          {columns.map((column) => (
            <span key={column}>{column}</span>
          ))}
        </div>
        {rows.map((row, index) => (
          <div className="grid-row" key={row[0]}>
            {row.map((cell, cellIndex) => (
              <span
                key={`${row[0]}-${cellIndex}`}
                className={
                  cell === "edited"
                    ? "cell-edited"
                    : cell === "new"
                      ? "cell-new"
                      : ""
                }
              >
                {cell}
              </span>
            ))}
            <i style={{ width: `${18 + index * 7}%` }} />
          </div>
        ))}
      </div>
    </div>
  );
}
