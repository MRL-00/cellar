import type { Engine } from "./EngineBadge";
import azure from "../assets/engines/azure.svg";
import convex from "../assets/engines/convex.svg";
import firestore from "../assets/engines/firestore.svg";
import mssql from "../assets/engines/mssql.svg";
import mysql from "../assets/engines/mysql.svg";
import neon from "../assets/engines/neon.svg";
import planetscale from "../assets/engines/planetscale.svg";
import postgres from "../assets/engines/postgres.svg";
import sqlite from "../assets/engines/sqlite.svg";
import supabase from "../assets/engines/supabase.svg";

// Full-color brand marks for picker-sized surfaces. The tiny sidebar badges
// keep the monochrome tinted glyphs in EngineBadge — multi-color marks are
// unreadable at 12px and can't inherit the connection accent.
const LOGOS: Record<Engine, string> = {
  postgres,
  mysql,
  sqlite,
  mssql,
  azure,
  firestore,
  convex,
  supabase,
  neon,
  planetscale,
};

export function EngineLogo({
  engine,
  size = 20,
}: {
  engine: Engine;
  size?: number;
}) {
  return (
    <img
      src={LOGOS[engine]}
      width={size}
      height={size}
      alt=""
      aria-hidden
      style={{ display: "block", objectFit: "contain" }}
    />
  );
}
