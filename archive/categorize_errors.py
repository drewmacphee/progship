#!/usr/bin/env python3
"""Categorize door errors by type."""
import re

rooms = {}
with open('rooms_dump.txt', 'r', encoding='utf-8') as f:
    for line in f:
        m = re.match(r'^\s*(\d+)\s*\|\s*(\d+)\s*\|\s*(-?\d+)\s*\|\s*([\d.]+)\s*\|\s*([\d.]+)\s*\|\s*([\d.]+)\s*\|\s*([\d.]+)\s*$', line.strip())
        if m:
            rooms[int(m.group(1))] = {'type': int(m.group(2)), 'deck': int(m.group(3)),
                'x': float(m.group(4)), 'y': float(m.group(5)),
                'w': float(m.group(6)), 'h': float(m.group(7))}

doors = []
with open('doors_dump.txt', 'r', encoding='utf-8') as f:
    for line in f:
        m = re.match(r'^\s*(\d+)\s*\|\s*(\d+)\s*\|\s*(\d+)\s*\|\s*(\d+)\s*\|\s*(\d+)\s*\|\s*([\d.]+)\s*\|\s*([\d.]+)\s*\|\s*([\d.]+)\s*$', line.strip())
        if m:
            doors.append({'id': int(m.group(1)), 'ra': int(m.group(2)), 'rb': int(m.group(3)),
                'wa': int(m.group(4)), 'wb': int(m.group(5)),
                'dx': float(m.group(6)), 'dy': float(m.group(7)), 'dw': float(m.group(8))})

WALL = {0: 'N', 1: 'S', 2: 'E', 3: 'W'}

def wall_coord(r, w):
    if w == 0: return r['y'] - r['h'] / 2
    if w == 1: return r['y'] + r['h'] / 2
    if w == 2: return r['x'] + r['w'] / 2
    if w == 3: return r['x'] - r['w'] / 2
    return 0

svc_cross = 0
shaft_cross = 0
orphan_list = []

for d in doors:
    ra, rb = rooms.get(d['ra']), rooms.get(d['rb'])
    if not ra or not rb:
        continue
    if ra['deck'] != rb['deck']:
        continue
    ca = wall_coord(ra, d['wa'])
    cb = wall_coord(rb, d['wb'])
    gap = abs(ca - cb)
    if gap > 1.5:
        ta, tb = ra['type'], rb['type']
        types = {ta, tb}
        if 101 in types and 102 in types:
            svc_cross += 1
        elif 110 in types or 111 in types:
            shaft_cross += 1
        else:
            orphan_list.append((d, ta, tb, gap))

print(f"Service-corridor <-> cross-corridor misaligned: {svc_cross}")
print(f"Shaft <-> cross-corridor misaligned: {shaft_cross}")
print(f"Other misaligned (orphan/force-connect): {len(orphan_list)}")

print("\nFirst 10 'other' misaligned doors:")
for d, ta, tb, gap in orphan_list[:10]:
    ra_r = rooms[d['ra']]
    rb_r = rooms[d['rb']]
    print(f"  Door {d['id']}: room {d['ra']}(type={ta},x={ra_r['x']},y={ra_r['y']},w={ra_r['w']}) "
          f"{WALL[d['wa']]} -> room {d['rb']}(type={tb},x={rb_r['x']},y={rb_r['y']},w={rb_r['w']}) "
          f"{WALL[d['wb']]}, gap={gap:.1f}")

# Analyze cross-corridor dimensions
print("\nCross-corridor rooms and their overlapping neighbors:")
for r in sorted(rooms.values(), key=lambda x: x.get('type', 0)):
    if r['type'] == 102 and r.get('deck', -1) == 0:
        print(f"  Room {[k for k,v in rooms.items() if v is r][0]} (cross-corridor): "
              f"x={r['x']}, y={r['y']}, w={r['w']}, h={r['h']}")
        print(f"    X range: [{r['x']-r['w']/2}, {r['x']+r['w']/2}]")

for r in sorted(rooms.values(), key=lambda x: x.get('type', 0)):
    if r['type'] == 101 and r.get('deck', -1) == 0:
        print(f"  Room {[k for k,v in rooms.items() if v is r][0]} (service corridor): "
              f"x={r['x']}, y={r['y']}, w={r['w']}, h={r['h']}")
        print(f"    X range: [{r['x']-r['w']/2}, {r['x']+r['w']/2}]")
