import json
from pathlib import Path

path = Path(__file__).resolve().parent.parent / '.fixtures' / 'connections.json'
configs = json.loads(path.read_text())
if any(config['id'] == 'demo-postgres' for config in configs):
    raise SystemExit('Postgres fixture alias already exists.')
configs.append(dict(
    id='demo-postgres', name='Cellar demo · Postgres', engine='postgres',
    host='127.0.0.1', port=55439, database='postgres', user='cellar_reader',
    ssl_mode='disable', env_tag='local', application_name=None, color='#4f8ff7'
))
path.write_text(json.dumps(configs, indent=2))
print('Added only the disposable localhost Postgres fixture alias.')
