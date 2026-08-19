import json

def load_json(path):
    with open(path, 'r', encoding='utf-8') as f:
        return json.load(f)

def save_json(path, data):
    with open(path, 'w', encoding='utf-8') as f:
        json.dump(data, f, indent=2, ensure_ascii=False)
        f.write('\n')

en = load_json('web/src/i18n/locales/en/setup.json')
pt = load_json('web/src/i18n/locales/pt-BR/setup.json')

def sync(en_dict, pt_dict):
    for k, v in en_dict.items():
        if isinstance(v, dict):
            if k not in pt_dict or not isinstance(pt_dict[k], dict):
                pt_dict[k] = {}
            sync(v, pt_dict[k])
        else:
            if k not in pt_dict:
                pt_dict[k] = v

sync(en, pt)
save_json('web/src/i18n/locales/pt-BR/setup.json', pt)
print("done")
