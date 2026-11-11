#!/usr/bin/env python3
# -*- coding: utf-8 -*-
"""Offline normalizer for Labanyu reference data. No network.

usage: build.py CACHE_DIR

CACHE_DIR holds what fetch.sh wrote: raw/*.next.json (extracted __NEXT_DATA__) and
requests.log. Step 1 writes CACHE_DIR/out/*.json (full normalized files, outside the
repo). Step 2 writes the repo copies data/reference/labanyu/<kind>.json: minified,
whitelisted fields only (no price/time/site ids). Price-like keys abort the build.
"""
import json, os, re, sys, collections

if len(sys.argv) != 2:
    sys.exit('usage: build.py CACHE_DIR')
BASE = os.path.abspath(sys.argv[1])
REPO_OUT = os.path.join(os.path.dirname(os.path.abspath(__file__)), '..', '..', 'data', 'reference', 'labanyu')
RAW, OUT = os.path.join(BASE, 'raw'), os.path.join(BASE, 'out')
SITE = 'https://mabi.labanyu.com'
os.makedirs(OUT, exist_ok=True)

fetched = {}
for line in open(os.path.join(BASE, 'requests.log')):
    ts, code, url, iso = line.rstrip('\n').split('\t')
    fetched[url] = iso

def pp(name):
    return json.load(open(os.path.join(RAW, name + '.next.json'), encoding='utf-8'))['props']['pageProps']

def write(fname, kind, path, entries, notes):
    url = SITE + path
    doc = {'schema_version': 1, 'kind': kind, 'source': 'labanyu_reference', 'source_url': url,
           'fetched_at': fetched[url], 'attribution': 'mabi.labanyu.com', 'notes': notes,
           'entry_count': len(entries), 'entries': entries}
    with open(os.path.join(OUT, fname), 'w', encoding='utf-8') as f:
        json.dump(doc, f, ensure_ascii=False, indent=1)
    print(fname, kind, len(entries))

auc = pp('auction')

# --- categories
write('categories.json', 'auction_item_categories', '/in-game/auction',
      [{'name': x['label'], 'labanyu_id': x['value']} for x in auc['auctionItemCategories']],
      'Auction item category list from the Labanyu auction search select (auctionItemCategories). labanyu_id is the site-internal id, not a Nexon API id.')

# --- reforge
mw = pp('metalware_options')['metalwareOptionValues']
by_type = collections.defaultdict(list)
by_opt = collections.defaultdict(list)
ranges = []
for x in mw:
    r = {'equipment_type': x['typeName'], 'option_name': x['optionName'], 'min_level': x['minLevel'],
         'max_level': x['maxLevel'], 'limit_break_max_level': x['limitBreakMaxLevel']}
    r['range_text'] = f"{x['minLevel']}~{x['maxLevel']}" + (f" (한돌: {x['limitBreakMaxLevel']})" if x['limitBreakMaxLevel'] is not None else '')
    ranges.append(r)
    by_type[x['typeName']].append(x['optionName'])
    by_opt[x['optionName']].append({'equipment_type': x['typeName'], 'range_text': r['range_text'],
                                    'min_level': x['minLevel'], 'max_level': x['maxLevel'],
                                    'limit_break_max_level': x['limitBreakMaxLevel']})
ranges.sort(key=lambda r: (r['equipment_type'], r['option_name']))
write('reforge_ranges.json', 'reforge_level_ranges', '/metalware/options', ranges,
      "One entry per (equipment_type, option_name) from /metalware/options (metalwareOptionValues, 2805 rows). min_level~max_level is the normal reforge range; limit_break_max_level is the 한돌 (limit-break) cap. Price field dropped.")

write('reforge_equipment_types.json', 'reforge_equipment_types', '/in-game/auction',
      [{'name': x['label'], 'labanyu_id': x['value'], 'option_count': len(by_type.get(x['label'], []))}
       for x in auc['metalwareitemTypes']],
      "Reforge (세공) equipment types from the auction select metalwareitemTypes (60). option_count = number of reforge options for this type in /metalware/options (reforge_ranges.json). All 60 types appear in /metalware/options.")

write('reforge_options.json', 'reforge_option_names', '/in-game/auction',
      [{'name': x['label'], 'labanyu_id': x['value'],
        'by_equipment_type': sorted(by_opt.get(x['label'], []), key=lambda r: r['equipment_type'])}
       for x in auc['metalwareOptionNameOptions']],
      "Reforge option names from the auction select metalwareOptionNameOptions (527). by_equipment_type joined from /metalware/options (fetched separately, see reforge_ranges.json source_url); every one of the 527 names has >=1 equipment-type range.")

# --- echostone (auction select 'echostoneProbabilities' == /echostone/option-price list)
STONE = {1: '레드 에코스톤', 2: '블루 에코스톤', 3: '옐로 에코스톤', 4: '실버 에코스톤', 5: '블랙 에코스톤'}
eco = pp('echostone_option_price')['optionPriceDatas']
write('echostone_effects.json', 'echostone_awakening_abilities', '/echostone/option-price',
      [{'name': x['name'], 'max_level': x['maxLevel'], 'labanyu_id': x['id'],
        'echostone_type_id': x['type'], 'echostone_type': STONE.get(x['type'])} for x in eco],
      "Echostone awakening abilities (에코스톤 각성 능력) with max level, from /echostone/option-price (optionPriceDatas, 580). echostone_type mapping (1 red..5 black) taken from the site JS chunk 2928. Same 580 names appear in the auction select previously mislabelled 'totems' (selects key 7, '(최대:N)'). Per-level price/check/update fields dropped. Names may repeat across stone colors.")

# --- murias relic effects (the 30-item list previously labelled 'echostone effects')
mre = []
for x in auc['muriasRelicOptionNames']:
    m = re.match(r'^(.*?)\s*\(최대\s*([0-9.]+)\s*(%|초)?\)$', x['label'])
    mre.append({'name': m.group(1) if m else x['label'], 'max': float(m.group(2)) if m and '.' in m.group(2) else (int(m.group(2)) if m else None),
                'max_unit': (m.group(3) or '') if m else None, 'label': x['label'],
                'abbreviation': x.get('summarizeText'), 'labanyu_id': x['value']})
write('murias_relic_effects.json', 'murias_relic_effects', '/in-game/auction', mre,
      "무리아스 유물 효과 30종 from the auction select muriasRelicOptionNames (the '에코스톤' 각성 select in the auction UI uses this list; prior notes called it echostone effects key 8). max parsed from '(최대 N%|초)'. abbreviation = site search shorthand (Labanyu curation).")

# --- totems
tot_dict = pp('totem_price')['totemMinPriceDataDict']
tkinds = []
for kind, abil in tot_dict.items():
    tkinds.append({'totem_kind': kind, 'abilities': [
        {'name': a, 'thresholds': sorted({r['targetValue'] for r in rows}),
         'max_threshold': max(r['targetValue'] for r in rows)} for a, rows in abil.items()]})
tot_entries = [{'name': x['label'], 'labanyu_id': x['value'],
                'kinds': [k['totem_kind'] for k in tkinds if any(a['name'] == x['label'] for a in k['abilities'])]}
               for x in auc['totemOptions']]
tdoc_notes = ("Totem ability names (61) from auction select totemOptions. 'kinds' lists totem kinds (애뮬릿/일반 토템/학회 주화) whose /totem/price table includes the ability. "
              "totem_kinds[]: per kind, abilities with the threshold values the site tracks (targetValue) — max_threshold is the highest tracked value, a proxy for max_level, NOT a confirmed game cap. All prices/check values/times dropped. "
              "Auction select has no max level for totems; the '(최대:N)' list of 580 is echostone (see echostone_effects.json).")
write('totems.json', 'totem_abilities', '/in-game/auction', tot_entries, tdoc_notes)
# attach kinds with their own source
d = json.load(open(os.path.join(OUT, 'totems.json'), encoding='utf-8'))
d['totem_kinds'] = tkinds
d['totem_kinds_source_url'] = SITE + '/totem/price'
d['totem_kinds_fetched_at'] = fetched[SITE + '/totem/price']
json.dump(d, open(os.path.join(OUT, 'totems.json'), 'w', encoding='utf-8'), ensure_ascii=False, indent=1)

# --- set bonus names (cheap extra)
write('set_bonus_names.json', 'set_bonus_names', '/in-game/auction',
      [{'name': x['label'], 'labanyu_id': x['value']} for x in auc['setBonusNames']],
      'Set-bonus (세트 효과) names from auction select setBonusNames.')

# --- enchants (port of Labanyu's slot-tree classification from pages/enchant/price chunk)
ep = pp('enchant_price')
groups = {g['id']: {'dungeon': g['name'], 'equipment_type': g['equipmentType'], 'equipment_level': g['equipmentLevel']}
          for g in ep['enchantScrollGroups']}
EXCLUDED = ["광신도의 대검","구원의 활","금속재질의 무기와 방어구","그레이스 플레이트 세트","노르만 워리어 세트","더스틴 실버 나이트 세트","실버스파이카 세트","탐구생활 낚싯대","탐구생활 식칼","탐구생활 장비","탐구생활 조련 지팡이","피닉스 깃털검","시양양과 친구들의 모자","디바인 스몰 랜스","디바인 트라이볼트 원드","마족표 무기","요리사 모자","마족표 근접 무기","마족표 실린더","마족표 양손무기","마족표 원드","마족표 장거리 무기","복음의 실린더","성흔의 너클","순례자의 검","승리의 세이버 검","신비한 피닉스 깃털검","아처의 활","여신에게 바쳐진 장검","원조의 스태프","지원용 던전 체인 블레이드","거대 반죽용 밀대","거대국자","거대식칼","귤","기억의 브로치","디바인 바스타드 소드","디바인 블레이드","디바인 신시엘라크 L2","디바인 실드","디바인 크로스보우","바나나","아이스크림","왕정 연금술사 부츠","왕정 연금술사 제복","트롤 몽둥이","파","특별 이벤트 장비 페러시우스 페러그린 사이드","팔라라의 조각","피닉스 귀마개","소재가 나무인 무기와 방어구","소재가 철인 무기와 방어구","소재가 나무가 아닌 건틀렛","철소재(방패불가) 아이템","거대한 요리 도구","간이테이블","냄비","요리용 도구","옷(갑옷 제외)","가죽 소재의 갑옷","천옷(마법의상 불가)","철제방어구","마법의상","원죄의 스태프","소형 도끼","천모자","관악기","현악기","전자기타"]
TREE = [
    ('무기', lambda e: re.search('무기|검|활|원드|스태프|실린더|너클|석궁|랜스|핸들|듀얼건|수리검|체인|낫|도끼|둔기', e)),
    ('악기', lambda e: e in ('악기', '관악기', '현악기', '전자기타')),
    ('보주 무기', lambda e: e in ('방패', '마도서', '오브')),
    ('옷, 갑옷', lambda e: re.search('갑옷|의상|로브|천옷|가죽', e) or e == '옷'),
    ('머리', lambda e: re.search('머리|투구|모자', e)),
    ('손', lambda e: e in ('손', '장갑', '건틀렛')),
    ('발', lambda e: e in ('발', '신발', '금속부츠')),
    ('악세사리', lambda e: re.search('장신구|악세|액세', e)),
    ('세트아이템 & 기타', lambda e: re.search('세트', e)),
]
def norm_target(e):
    if '머리에 착용하는' in e: return ['머리']
    if '손에 착용하는' in e: return ['손']
    if '발에 착용하는' in e: return ['발']
    if re.sub(r'\s', '', e) == '천옷': return ['천옷']
    t = []
    if '관악기' in e: t.append('관악기')
    if '현악기' in e: t.append('현악기')
    if '전자기타' in e or '전자 기타' in e or '전자악기' in e: t.append('전자기타')
    if t: return t
    if e == '악기' or '악기에 착용' in e: return ['악기']
    for k in ('건틀렛', '장갑'):
        if k in e: return [k]
    if '금속부츠' in re.sub(r'\s', '', e): return ['금속부츠']
    for k in ('신발', '마도서', '오브', '방패'):
        if k in e: return [k]
    return [e]
def classify(targets):
    if not targets:  # site: empty targetNames == "장비 제한 없음" chip
        return [{'group': '장비 제한 없음', 'slot': '장비 제한 없음'}]
    out = []
    for tn in targets:
        if tn in EXCLUDED: continue
        for s in norm_target(tn):
            if s in ('관악기', '현악기', '전자기타'): continue
            grp = next((title for title, f in TREE if f(s)), '기타')
            if {'group': grp, 'slot': s} not in out: out.append({'group': grp, 'slot': s})
    return out

ench = []
for e in ep['enchantScrolls']:
    info = [l.strip() for l in (e['info'] or '').split('\n') if l.strip()]
    ench.append({'name': e['name'], 'slot': e['type'], 'slot_en': {'접두': 'prefix', '접미': 'suffix'}.get(e['type']),
                 'rank': e['rank'].replace(' 랭크', ''), 'rank_label': e['rank'], 'rank_order': e['rankId'],
                 'labanyu_id': e['id'], 'effect_lines': info, 'stats': e['stats'] or [],
                 'target_names': e['targetNames'] or [],
                 'dungeon_grade': groups.get(e['groupId']),
                 'equipment_slot_classification': classify(e['targetNames'])})
write('enchants.json', 'enchant_scrolls', '/enchant/price', ench,
      "Enchant scrolls (1185 rows; 1169 unique (slot,name) — duplicates are distinct site ids with different targets/effects) from /enchant/price __NEXT_DATA__. "
      "Game facts: name, slot (접두/접미), rank, effect_lines (site text, first line usually '<target>에 인챈트 가능'), stats (site-parsed numeric stats), target_names. "
      "Labanyu curation: dungeon_grade (enchantScrollGroups; site says grades are 'arbitrarily assigned' and the enchant may not drop there; null for 1025 rows) and equipment_slot_classification "
      "(Python port of the site's JS slot-tree; targets in the site's exclusion list (named items/sets) yield no slot). rank_order is the site's rankId (2=F ... 16=1). Price/time dropped.")

# ---------- step 2: repo copies (data/reference/labanyu/<kind>.json) ----------
# SOT: reforge ranges live only in reforge_level_ranges; enchant rank order comes from
# the domain EnchantRank (rank_order/rank_label/slot_en dropped); site ids dropped.
def pick(*keys):
    return lambda e: {k: e[k] for k in keys}

def totem_entry(e, kinds):
    return {'name': e['name'], 'kinds': [
        {'totem_kind': k['totem_kind'], 'max_threshold': a['max_threshold']}
        for k in kinds for a in k['abilities'] if a['name'] == e['name']]}

EXPORT = [
    ('categories.json', pick('name')),
    ('reforge_equipment_types.json', pick('name', 'option_count')),
    ('reforge_options.json', pick('name')),
    ('reforge_ranges.json', pick('equipment_type', 'option_name', 'min_level', 'max_level',
                                 'limit_break_max_level', 'range_text')),
    ('totems.json', None),
    ('echostone_effects.json', pick('name', 'max_level', 'echostone_type')),
    ('murias_relic_effects.json', pick('name', 'max', 'max_unit', 'abbreviation')),
    ('set_bonus_names.json', pick('name')),
    ('enchants.json', pick('name', 'slot', 'rank', 'effect_lines', 'stats', 'target_names',
                           'dungeon_grade', 'equipment_slot_classification')),
]
FORBIDDEN = re.compile(r'price|가격|volume|거래량|seller|buyer|character|user|time', re.I)

def keys(v):
    if isinstance(v, dict):
        for k, x in v.items():
            yield k
            yield from keys(x)
    elif isinstance(v, list):
        for x in v:
            yield from keys(x)

os.makedirs(REPO_OUT, exist_ok=True)
for fname, f in EXPORT:
    src = json.load(open(os.path.join(OUT, fname), encoding='utf-8'))
    if fname == 'totems.json':
        entries = [totem_entry(e, src['totem_kinds']) for e in src['entries']]
        notes = (src['notes'].split(' totem_kinds[]')[0] +
                 " kinds[].max_threshold: highest threshold tracked on " + src['totem_kinds_source_url'] +
                 " (fetched " + src['totem_kinds_fetched_at'] + "), a proxy for max level, NOT a confirmed game cap.")
    else:
        entries = [f(e) for e in src['entries']]
        notes = src['notes']
    if fname == 'reforge_options.json':
        notes = 'Reforge option names from the auction select metalwareOptionNameOptions (527). Ranges per equipment type: see reforge_level_ranges.'
    if fname == 'enchants.json':
        notes = notes.replace(' rank_order is the site\'s rankId (2=F ... 16=1).', '')
    doc = {'schema_version': 1, 'kind': src['kind'], 'source': 'labanyu_reference',
           'source_url': src['source_url'], 'fetched_at': src['fetched_at'],
           'attribution': 'mabi.labanyu.com', 'notes': notes, 'entries': entries}
    bad = sorted({k for k in keys(entries) if FORBIDDEN.search(k)})
    if bad:
        sys.exit(f'{fname}: forbidden keys {bad}')
    path = os.path.join(REPO_OUT, src['kind'] + '.json')
    with open(path, 'w', encoding='utf-8') as fh:
        json.dump(doc, fh, ensure_ascii=False, separators=(',', ':'))
        fh.write('\n')
    print('repo', src['kind'], len(entries))
