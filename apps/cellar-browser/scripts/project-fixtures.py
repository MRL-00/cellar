"""Generate only disposable synthetic project configuration for discovery tests."""
import json
import shutil
from pathlib import Path

app = Path(__file__).resolve().parent.parent
root = app / '.fixtures' / 'project-demo'
if root.exists():
    raise SystemExit('Project fixture already exists; remove only .fixtures/project-demo to regenerate.')
(root / 'data').mkdir(parents=True, exist_ok=True)
shutil.copyfile(app / '.fixtures/demo.sqlite', root / 'data/demo.sqlite')
(root / '.env').write_text('DATABASE_URL=postgresql://cellar_reader:synthetic-password@127.0.0.1:55440/postgres\nSQLITE_PATH=data/demo.sqlite\n')
(root / '.env.production').write_text('DATABASE_URL=postgresql://reader:synthetic-only@prod.example.test/example\n')
(root / '.env.development').write_text('DATABASE_URL=postgresql://${DB_USER}:${DB_PASSWORD}@localhost/example\n')
(root / 'appsettings.json').write_text(json.dumps({'ConnectionStrings': {'LocalSQLite': 'Data Source=data/demo.sqlite'}}))
print('Generated disposable project fixtures; no credentials were imported.')
