#!/usr/bin/env python3
"""One-time local-site migration: current builds/media plus a consistent SQLite backup, never browser sessions.
Does not change the source database. Deployment credentials stay outside the archive and source tree.
"""
import argparse
import base64
import hashlib
import json
import os
from pathlib import Path
import secrets
import sqlite3
import subprocess

parser = argparse.ArgumentParser(description=__doc__)
parser.add_argument('--out', required=True, type=Path, help='New staging directory outside the source data directory')
parser.add_argument('--credentials', required=True, type=Path, help='Private local file for the deployed demo login')
args = parser.parse_args()
source = Path(__file__).resolve().parents[1] / 'data'
out = args.out.resolve()
if out == source or source in out.parents:
    parser.error('Choose a staging directory outside the live data directory')
out.mkdir(parents=True, exist_ok=False)
os.chmod(out, 0o700)
with sqlite3.connect(f'file:{source / "site.sqlite"}?mode=ro', uri=True) as original:
    with sqlite3.connect(out / 'site.sqlite') as copy:
        original.backup(copy)
        copy.execute('DELETE FROM sessions')
        copy.execute("DELETE FROM users WHERE handle LIKE 'smoke_%' AND email = handle || '@gigacouch.test' AND display_name IN ('Smoke Test', 'Smoke Tester') AND id NOT IN (SELECT owner_id FROM games)")
        # Migrate exactly the live library. Historical local builds remain available in the original data directory.
        copy.execute('DELETE FROM builds WHERE id NOT IN (SELECT build_id FROM games WHERE build_id IS NOT NULL)')
        if copy.execute("SELECT 1 FROM users WHERE handle='demo'").fetchone():
            password = secrets.token_urlsafe(32)
            salt = secrets.token_bytes(16)
            hashed = hashlib.scrypt(password.encode(), salt=salt, n=16384, r=8, p=1, dklen=64)
            record = 'scrypt$' + base64.b64encode(salt).decode() + '$' + base64.b64encode(hashed).decode()
            copy.execute("UPDATE users SET password_hash=? WHERE handle='demo'", (record,))
            args.credentials.parent.mkdir(parents=True, exist_ok=True)
            fd = os.open(args.credentials, os.O_WRONLY | os.O_CREAT | os.O_EXCL, 0o600)
            with os.fdopen(fd, 'w') as credential:
                credential.write('Site: https://gigacouch-platform.fly.dev\nHandle: demo\nPassword: ' + password + '\n')
        builds = [row[0] for row in copy.execute('SELECT build_id FROM games WHERE build_id IS NOT NULL')]
        games = copy.execute('SELECT count(*) FROM games').fetchone()[0]
        copy.commit()
        copy.execute('PRAGMA journal_mode=DELETE')
        assert copy.execute('PRAGMA integrity_check').fetchone()[0] == 'ok'
os.chmod(out / 'site.sqlite', 0o600)
for build in builds:
    if not (source / 'builds' / build / 'index.html').is_file():
        raise RuntimeError(f'Missing live build {build}')
archive = out / 'site.tar.gz'
subprocess.run(['tar', '-czf', str(archive), '-C', str(out), 'site.sqlite', '-C', str(source),
                *['builds/' + build for build in builds], 'media'], check=True)
os.chmod(archive, 0o600)
print(json.dumps({'games': games, 'builds': builds, 'archive': str(archive), 'bytes': archive.stat().st_size}))
