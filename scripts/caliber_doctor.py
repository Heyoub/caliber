#!/usr/bin/env python3
import os
import shutil
import subprocess
import sys
import textwrap
from dataclasses import dataclass
import argparse
from typing import List, Optional, Tuple


@dataclass
class CmdResult:
    code: int
    out: str
    err: str


@dataclass
class Recommendation:
    title: str
    commands: List[str]
    reason: str


def run_cmd(cmd: List[str], env: Optional[dict] = None) -> CmdResult:
    proc = subprocess.run(
        cmd,
        stdout=subprocess.PIPE,
        stderr=subprocess.PIPE,
        text=True,
        env=env,
    )
    return CmdResult(proc.returncode, proc.stdout.strip(), proc.stderr.strip())


def is_tty() -> bool:
    return sys.stdin.isatty() and sys.stdout.isatty()


def read_key() -> str:
    import termios
    import tty

    fd = sys.stdin.fileno()
    old = termios.tcgetattr(fd)
    try:
        tty.setraw(fd)
        ch1 = sys.stdin.read(1)
        if ch1 == "\x1b":
            ch2 = sys.stdin.read(1)
            if ch2 == "[":
                ch3 = sys.stdin.read(1)
                return f"\x1b[{ch3}"
        return ch1
    finally:
        termios.tcsetattr(fd, termios.TCSADRAIN, old)


def arrow_menu(title: str, items: List[str]) -> int:
    if not is_tty():
        print(title)
        for i, item in enumerate(items, start=1):
            print(f"  {i}) {item}")
        while True:
            try:
                choice = int(input("Select: ").strip())
                if 1 <= choice <= len(items):
                    return choice - 1
            except ValueError:
                pass
            print("Invalid selection.")

    idx = 0
    while True:
        sys.stdout.write("\x1b[2J\x1b[H")
        print(title)
        for i, item in enumerate(items):
            prefix = "➤ " if i == idx else "  "
            print(f"{prefix}{item}")
        print("\nUse ↑/↓, Enter to select, q to quit.")
        key = read_key()
        if key in ("\x1b[A", "k"):
            idx = (idx - 1) % len(items)
        elif key in ("\x1b[B", "j"):
            idx = (idx + 1) % len(items)
        elif key in ("\r", "\n"):
            return idx
        elif key in ("q", "\x03"):
            return len(items) - 1


def env_default(key: str, default: str) -> str:
    val = os.environ.get(key)
    if val is None or val == "":
        return default
    return val


def is_local_db(host: str) -> bool:
    return host in ("", "localhost", "127.0.0.1", "::1")


def psql_cmd(user: str, password: str, host: str, port: str, db: str, sql: str) -> Tuple[List[str], dict]:
    env = os.environ.copy()
    if password:
        env["PGPASSWORD"] = password
    cmd = ["psql", "-v", "ON_ERROR_STOP=1", "-h", host, "-p", port, "-U", user, "-d", db, "-c", sql]
    return cmd, env


def detect_paths() -> dict:
    cargo = shutil.which("cargo") or ""
    psql = shutil.which("psql") or ""
    return {"cargo": cargo, "psql": psql}


def diagnose() -> Tuple[dict, List[str]]:
    db_host = env_default("CALIBER_DB_HOST", "localhost")
    db_port = env_default("CALIBER_DB_PORT", "5432")
    db_name = env_default("CALIBER_DB_NAME", "caliber")
    db_user = env_default("CALIBER_DB_USER", "caliber")
    db_pass = env_default("CALIBER_DB_PASSWORD", "")
    bootstrap_user_env = os.environ.get("CALIBER_DB_BOOTSTRAP_USER")
    bootstrap_pass_env = os.environ.get("CALIBER_DB_BOOTSTRAP_PASSWORD")
    bootstrap_user = env_default("CALIBER_DB_BOOTSTRAP_USER", db_user)
    bootstrap_pass = env_default("CALIBER_DB_BOOTSTRAP_PASSWORD", db_pass)
    bootstrap_explicit = bootstrap_user_env is not None or bootstrap_pass_env is not None

    paths = detect_paths()
    notes: List[str] = []
    results = {
        "db": {
            "host": db_host,
            "port": db_port,
            "name": db_name,
            "user": db_user,
            "bootstrap_user": bootstrap_user,
            "is_local": is_local_db(db_host),
        },
        "paths": paths,
        "checks": {},
        "check_users": {},
    }

    if not paths["psql"]:
        notes.append("psql not found on PATH.")
        return results, notes

    # Basic connectivity as app user.
    cmd, env = psql_cmd(db_user, db_pass, db_host, db_port, db_name, "select 1;")
    res = run_cmd(cmd, env)
    results["checks"]["db_user_connect"] = res
    if res.code != 0:
        notes.append("DB user connection failed.")

    bootstrap_note_added = False

    def run_check(key: str, sql: str) -> None:
        nonlocal bootstrap_note_added
        # Prefer bootstrap only if explicitly set; otherwise use app user.
        preferred_user = bootstrap_user if bootstrap_explicit else db_user
        preferred_pass = bootstrap_pass if bootstrap_explicit else db_pass
        cmd, env = psql_cmd(preferred_user, preferred_pass, db_host, db_port, db_name, sql)
        res = run_cmd(cmd, env)
        if res.code != 0 and bootstrap_explicit and preferred_user != db_user:
            # Fall back to app user for read-only checks.
            cmd, env = psql_cmd(db_user, db_pass, db_host, db_port, db_name, sql)
            res_fallback = run_cmd(cmd, env)
            if res_fallback.code == 0:
                # Avoid repeating the same note for every check.
                if not bootstrap_note_added:
                    notes.append("Bootstrap auth failed; used app user for read-only checks.")
                    bootstrap_note_added = True
                results["checks"][key] = res_fallback
                results["check_users"][key] = db_user
                return
        results["checks"][key] = res
        results["check_users"][key] = preferred_user

    # Extension version.
    run_check("ext_version", "select extname, extversion from pg_extension where extname = 'caliber_pg';")

    # Function exists?
    run_check("fn_agent_register",
              "select 1 from pg_proc p join pg_namespace n on n.oid=p.pronamespace "
              "where n.nspname='public' and p.proname='caliber_agent_register';")

    # Table owned by extension?
    run_check("table_owner",
              "select c.relname, e.extname from pg_class c "
              "left join pg_depend d on d.objid=c.oid and d.deptype='e' "
              "left join pg_extension e on e.oid=d.refobjid "
              "where c.relname='caliber_agent';")

    # pgvector available?
    run_check("pgvector_available",
              "select 1 from pg_available_extensions where name = 'vector';")

    return results, notes


def build_recommendations(state: dict, notes: List[str]) -> List[Recommendation]:
    db = state["db"]
    paths = state["paths"]
    checks = state["checks"]
    recs: List[Recommendation] = []

    if not paths["cargo"]:
        recs.append(
            Recommendation(
                "Cargo not on PATH for sudo",
                [
                    "sudo -E env \"PATH=$PATH\" \"RUSTUP_TOOLCHAIN=stable\" \"$(command -v cargo)\" "
                    "pgrx install --package caliber-pg --pg-config \"/usr/lib/postgresql/18/bin/pg_config\""
                ],
                "sudo env needs PATH to find cargo.",
            )
        )

    if "db_user_connect" in checks and checks["db_user_connect"].code != 0:
        recs.append(
            Recommendation(
                "Fix DB user connectivity",
                [
                    "export CALIBER_DB_USER=caliber",
                    "export CALIBER_DB_PASSWORD=your_password",
                    "psql -h {host} -p {port} -U {user} -d {db} -c \"select 1;\"".format(
                        host=db["host"], port=db["port"], user=db["user"], db=db["name"]
                    ),
                ],
                "App user could not connect.",
            )
        )

    if "pgvector_available" in checks and checks["pgvector_available"].code != 0:
        if db["is_local"]:
            recs.append(
                Recommendation(
                    "Enable pgvector (local Postgres)",
                    [
                        "sudo -u postgres psql -d {db} -c \"CREATE EXTENSION IF NOT EXISTS vector;\"".format(
                            db=db["name"]
                        )
                    ],
                    "pgvector missing or not available.",
                )
            )

    fn_ok = "fn_agent_register" in checks and "1" in checks["fn_agent_register"].out
    ext_ok = "ext_version" in checks and "caliber_pg" in checks["ext_version"].out
    if ext_ok and not fn_ok:
        recs.append(
            Recommendation(
                "Reinstall extension (functions missing)",
                [
                    "sudo -u postgres psql -d {db} -c \"DROP EXTENSION IF EXISTS caliber_pg CASCADE;\"".format(
                        db=db["name"]
                    ),
                    "sudo -E env \"PATH=$PATH\" \"RUSTUP_TOOLCHAIN=stable\" \"$(command -v cargo)\" "
                    "pgrx install --package caliber-pg --pg-config \"/usr/lib/postgresql/18/bin/pg_config\"",
                    "sudo -u postgres psql -d {db} -c \"CREATE EXTENSION caliber_pg;\"".format(
                        db=db["name"]
                    ),
                ],
                "Extension installed without generated SQL (functions missing).",
            )
        )

    if not ext_ok and db["is_local"]:
        recs.append(
            Recommendation(
                "Install extension fresh",
                [
                    "sudo -E env \"PATH=$PATH\" \"RUSTUP_TOOLCHAIN=stable\" \"$(command -v cargo)\" "
                    "pgrx install --package caliber-pg --pg-config \"/usr/lib/postgresql/18/bin/pg_config\"",
                    "sudo -u postgres psql -d {db} -c \"CREATE EXTENSION caliber_pg;\"".format(
                        db=db["name"]
                    ),
                ],
                "caliber_pg extension not installed.",
            )
        )

    return recs


def format_summary(state: dict, notes: List[str]) -> str:
    db = state["db"]
    checks = state["checks"]
    check_users = state.get("check_users", {})
    lines = []
    lines.append("DB: {user}@{host}:{port}/{db} (local={local})".format(
        user=db["user"], host=db["host"], port=db["port"], db=db["name"], local=db["is_local"]
    ))

    if "ext_version" in checks:
        suffix = f" (as {check_users.get('ext_version', 'unknown')})"
        lines.append("extension: " + (checks["ext_version"].out or checks["ext_version"].err or "no result") + suffix)
    if "fn_agent_register" in checks:
        fn = "present" if "1" in checks["fn_agent_register"].out else "missing"
        suffix = f" (as {check_users.get('fn_agent_register', 'unknown')})"
        lines.append("caliber_agent_register: " + fn + suffix)
    if "table_owner" in checks:
        suffix = f" (as {check_users.get('table_owner', 'unknown')})"
        lines.append("caliber_agent owner: " + (checks["table_owner"].out or "n/a") + suffix)
    if "pgvector_available" in checks:
        suffix = f" (as {check_users.get('pgvector_available', 'unknown')})"
        lines.append("pgvector available: " + ("yes" if "1" in checks["pgvector_available"].out else "no") + suffix)
    if notes:
        lines.append("notes: " + "; ".join(notes))
    return "\n".join(lines)


def format_llm_paste(state: dict, notes: List[str]) -> str:
    checks = state["checks"]
    def block(name: str, res: CmdResult) -> str:
        return textwrap.dedent(f"""\
        ## {name}
        code: {res.code}
        out: {res.out}
        err: {res.err}
        """).strip()

    parts = ["# Caliber Doctor - LLM Paste"]
    parts.append(format_summary(state, notes))
    for key in ("ext_version", "fn_agent_register", "table_owner", "pgvector_available", "db_user_connect"):
        if key in checks:
            parts.append(block(key, checks[key]))
    return "\n\n".join(parts)


def main() -> int:
    parser = argparse.ArgumentParser(description="Caliber Doctor")
    parser.add_argument("--llm", action="store_true", help="print LLM paste block and exit")
    parser.add_argument("--no-menu", action="store_true", help="skip interactive menu")
    args = parser.parse_args()

    state, notes = diagnose()
    recs = build_recommendations(state, notes)

    summary = format_summary(state, notes)
    print(summary)
    print()
    if recs:
        print("Recommended actions:")
        for i, rec in enumerate(recs, start=1):
            print(f"{i}. {rec.title}")
            print(f"   Reason: {rec.reason}")
            for cmd in rec.commands:
                print(f"   - {cmd}")
    else:
        print("No immediate issues detected.")

    if args.llm:
        print()
        print(format_llm_paste(state, notes))
        return 0

    if args.no_menu:
        return 0

    menu = [
        "Print recommended commands",
        "Generate LLM paste block",
        "Run one recommended command",
        "Exit",
    ]
    choice = arrow_menu("Caliber Doctor", menu)

    if choice == 0:
        print()
        for rec in recs:
            print(f"{rec.title}")
            for cmd in rec.commands:
                print(cmd)
            print()
        return 0

    if choice == 1:
        print()
        print(format_llm_paste(state, notes))
        return 0

    if choice == 2:
        if not recs:
            print("No recommended commands to run.")
            return 0
        all_cmds: List[str] = []
        for rec in recs:
            all_cmds.extend(rec.commands)
        cmd_idx = arrow_menu("Select command to run", all_cmds + ["Cancel"])
        if cmd_idx >= len(all_cmds):
            return 0
        cmd = all_cmds[cmd_idx]
        print(f"\nRunning: {cmd}\n")
        res = run_cmd(["bash", "-lc", cmd], env=os.environ.copy())
        print(res.out)
        if res.err:
            print(res.err, file=sys.stderr)
        return res.code

    return 0


if __name__ == "__main__":
    raise SystemExit(main())
