#!/usr/bin/env python3
"""Mathematical verification of door placement, wall assignments, and traversal logic."""
import re

# Parse rooms
rooms = {}
with open('rooms_dump.txt', 'r', encoding='utf-8') as f:
    for line in f:
        line = line.strip()
        m = re.match(r'^\s*(\d+)\s*\|\s*(\d+)\s*\|\s*(-?\d+)\s*\|\s*([\d.]+)\s*\|\s*([\d.]+)\s*\|\s*([\d.]+)\s*\|\s*([\d.]+)\s*$', line)
        if m:
            rid = int(m.group(1))
            rooms[rid] = {
                'id': rid, 'type': int(m.group(2)), 'deck': int(m.group(3)),
                'x': float(m.group(4)), 'y': float(m.group(5)),
                'w': float(m.group(6)), 'h': float(m.group(7)),
            }

# Parse doors
doors = []
with open('doors_dump.txt', 'r', encoding='utf-8') as f:
    for line in f:
        line = line.strip()
        m = re.match(r'^\s*(\d+)\s*\|\s*(\d+)\s*\|\s*(\d+)\s*\|\s*(\d+)\s*\|\s*(\d+)\s*\|\s*([\d.]+)\s*\|\s*([\d.]+)\s*\|\s*([\d.]+)\s*$', line)
        if m:
            doors.append({
                'id': int(m.group(1)), 'room_a': int(m.group(2)), 'room_b': int(m.group(3)),
                'wall_a': int(m.group(4)), 'wall_b': int(m.group(5)),
                'door_x': float(m.group(6)), 'door_y': float(m.group(7)),
                'width': float(m.group(8)),
            })

print(f'Loaded {len(rooms)} rooms, {len(doors)} doors')

WALL_NAMES = {0: 'NORTH', 1: 'SOUTH', 2: 'EAST', 3: 'WEST'}

def room_edges(r):
    """Return (north_y, south_y, east_x, west_x) = (low_y, high_y, high_x, low_x)"""
    return (
        r['y'] - r['h']/2,  # north = low Y
        r['y'] + r['h']/2,  # south = high Y
        r['x'] + r['w']/2,  # east = high X
        r['x'] - r['w']/2,  # west = low X
    )

def wall_coord(r, wall):
    """Return (axis, coordinate) of the wall."""
    n, s, e, w = room_edges(r)
    if wall == 0: return ('y', n)
    elif wall == 1: return ('y', s)
    elif wall == 2: return ('x', e)
    elif wall == 3: return ('x', w)
    return ('?', 0)

errors = []
warnings = []

for d in doors:
    ra = rooms.get(d['room_a'])
    rb = rooms.get(d['room_b'])
    if not ra or not rb:
        errors.append(f"Door {d['id']}: room_a={d['room_a']} or room_b={d['room_b']} not found")
        continue

    dx, dy = d['door_x'], d['door_y']
    wa, wb = d['wall_a'], d['wall_b']
    same_deck = ra['deck'] == rb['deck']
    embedded_b = wb >= 200  # wall_b=255 means room_b has no wall gap (shaft embedded inside corridor)
    cross_deck = not same_deck  # Cross-deck doors are vertical shaft passages, skip wall checks

    # -- CHECK 1: Door coordinate matches room_a's wall (skip for cross-deck) --
    if not cross_deck:
        axis_a, coord_a = wall_coord(ra, wa)
        if axis_a == 'x':
            off = abs(dx - coord_a)
            if off > 1.0:
                errors.append(f"Door {d['id']}: door_x={dx} NOT on room_a({d['room_a']}) "
                              f"{WALL_NAMES.get(wa,'?')} wall at x={coord_a} (off by {off:.1f})")
        else:
            off = abs(dy - coord_a)
            if off > 1.0:
                errors.append(f"Door {d['id']}: door_y={dy} NOT on room_a({d['room_a']}) "
                              f"{WALL_NAMES.get(wa,'?')} wall at y={coord_a} (off by {off:.1f})")
    else:
        axis_a, coord_a = '?', 0

    # -- CHECK 2: Door coordinate matches room_b's wall (skip for embedded/cross-deck) --
    if not embedded_b and not cross_deck:
        axis_b, coord_b = wall_coord(rb, wb)
        if axis_b == 'x':
            off = abs(dx - coord_b)
            if off > 1.0:
                errors.append(f"Door {d['id']}: door_x={dx} NOT on room_b({d['room_b']}) "
                              f"{WALL_NAMES.get(wb,'?')} wall at x={coord_b} (off by {off:.1f})")
        else:
            off = abs(dy - coord_b)
            if off > 1.0:
                errors.append(f"Door {d['id']}: door_y={dy} NOT on room_b({d['room_b']}) "
                              f"{WALL_NAMES.get(wb,'?')} wall at y={coord_b} (off by {off:.1f})")
    else:
        axis_b, coord_b = '?', 0

    # -- CHECK 3: Door within room_a bounds along the wall (skip for cross-deck) --
    na, sa, ea, wa_x = room_edges(ra)
    nb, sb, eb, wb_x = room_edges(rb)

    if not cross_deck:
        if d['wall_a'] in (0, 1):  # N/S wall: door_x must be in room_a x range
            if dx < wa_x - 0.5 or dx > ea + 0.5:
                errors.append(f"Door {d['id']}: door_x={dx} outside room_a({d['room_a']}) "
                              f"x range [{wa_x:.1f}, {ea:.1f}]")
        else:  # E/W wall: door_y must be in room_a y range
            if dy < na - 0.5 or dy > sa + 0.5:
                errors.append(f"Door {d['id']}: door_y={dy} outside room_a({d['room_a']}) "
                              f"y range [{na:.1f}, {sa:.1f}]")

    # -- CHECK 4: Door within room_b bounds along the wall (skip for embedded/cross-deck) --
    if not embedded_b and not cross_deck:
        if d['wall_b'] in (0, 1):
            if dx < wb_x - 0.5 or dx > eb + 0.5:
                errors.append(f"Door {d['id']}: door_x={dx} outside room_b({d['room_b']}) "
                              f"x range [{wb_x:.1f}, {eb:.1f}]")
        else:
            if dy < nb - 0.5 or dy > sb + 0.5:
                errors.append(f"Door {d['id']}: door_y={dy} outside room_b({d['room_b']}) "
                              f"y range [{nb:.1f}, {sb:.1f}]")

    # -- CHECK 5: Walls are adjacent (same-deck doors share the wall coordinate, skip embedded) --
    if same_deck and not embedded_b:
        gap = abs(coord_a - coord_b)
        if gap > 1.5:
            errors.append(f"Door {d['id']}: walls NOT adjacent - "
                          f"room_a({d['room_a']}) {WALL_NAMES[d['wall_a']]} at {coord_a:.1f}, "
                          f"room_b({d['room_b']}) {WALL_NAMES[d['wall_b']]} at {coord_b:.1f} "
                          f"(gap={gap:.1f})")

    # -- CHECK 6: Wall pairing is consistent (skip for embedded) --
    valid_pairs = {(2,3), (3,2), (0,1), (1,0)}
    if same_deck and not embedded_b and (d['wall_a'], d['wall_b']) not in valid_pairs:
        warnings.append(f"Door {d['id']}: unusual wall pairing "
                        f"{WALL_NAMES[d['wall_a']]}/{WALL_NAMES[d['wall_b']]} "
                        f"between rooms {d['room_a']}/{d['room_b']}")

    # -- CHECK 7: Hull boundary --
    if dx < 0.5 or dy < 0.5:
        errors.append(f"Door {d['id']}: at hull boundary door_x={dx}, door_y={dy}")

    # -- CHECK 8: Simulate traversal from room_a to room_b --
    # New movement: player placed at door_x/door_y + small offset, clamped to room_b
    if same_deck:
        player_radius = 0.3
        half_w = rb['w']/2 - player_radius
        half_h = rb['h']/2 - player_radius

        # The offset direction depends on movement direction, but for verification
        # we just check the door position is near room_b's interior
        entry_x = max(rb['x'] - half_w, min(dx, rb['x'] + half_w))
        entry_y = max(rb['y'] - half_h, min(dy, rb['y'] + half_h))

        # Entry point should be near the door position
        dist = ((entry_x - dx)**2 + (entry_y - dy)**2)**0.5
        if dist > rb['w']/2 + rb['h']/2:
            errors.append(f"Door {d['id']}: TELEPORT! Entry in room_b({d['room_b']}) at "
                          f"({entry_x:.1f},{entry_y:.1f}) is {dist:.1f}m from door "
                          f"({dx},{dy})")
        elif dist > 10:
            warnings.append(f"Door {d['id']}: far entry in room_b({d['room_b']}) at "
                            f"({entry_x:.1f},{entry_y:.1f}), {dist:.1f}m from door "
                            f"({dx},{dy})")

# -- CHECK 9: Room overlap detection --
print("\n=== ROOM OVERLAP CHECK ===")
overlap_count = 0
deck_rooms = {}
for r in rooms.values():
    deck_rooms.setdefault(r['deck'], []).append(r)

for deck, rlist in sorted(deck_rooms.items()):
    for i in range(len(rlist)):
        for j in range(i+1, len(rlist)):
            a, b = rlist[i], rlist[j]
            na, sa, ea, wa2 = room_edges(a)
            nb2, sb2, eb2, wb2 = room_edges(b)
            # Check for overlap (not just touching)
            if wa2 < eb2 and wb2 < ea and na < sb2 and nb2 < sa:
                overlap_x = min(ea, eb2) - max(wa2, wb2)
                overlap_y = min(sa, sb2) - max(na, nb2)
                if overlap_x > 0.1 and overlap_y > 0.1:
                    overlap_count += 1
                    if overlap_count <= 20:
                        print(f"  Deck {deck}: Room {a['id']}(type={a['type']}) and "
                              f"Room {b['id']}(type={b['type']}) overlap by "
                              f"{overlap_x:.1f}x{overlap_y:.1f}m")

print(f"  Total overlapping room pairs: {overlap_count}")

# -- Print results --
print(f'\n=== ERRORS ({len(errors)}) ===')
for e in errors[:60]:
    print(f"  {e}")
if len(errors) > 60:
    print(f"  ... and {len(errors)-60} more")

print(f'\n=== WARNINGS ({len(warnings)}) ===')
for w in warnings[:30]:
    print(f"  {w}")
if len(warnings) > 30:
    print(f"  ... and {len(warnings)-30} more")

print(f'\nSUMMARY: {len(errors)} errors, {len(warnings)} warnings across {len(doors)} doors')

# -- CHECK 10: Connectivity - can player reach every room from spawn? --
print("\n=== CONNECTIVITY CHECK (Deck 0) ===")
deck0_rooms = {r['id'] for r in rooms.values() if r['deck'] == 0}
adj = {rid: set() for rid in deck0_rooms}
for d in doors:
    if d['room_a'] in deck0_rooms and d['room_b'] in deck0_rooms:
        adj[d['room_a']].add(d['room_b'])
        adj[d['room_b']].add(d['room_a'])

# BFS from first corridor (type 100)
start = None
for rid in sorted(deck0_rooms):
    if rooms[rid]['type'] == 100:
        start = rid
        break
if start:
    visited = set()
    queue = [start]
    visited.add(start)
    while queue:
        cur = queue.pop(0)
        for nb in adj.get(cur, []):
            if nb not in visited:
                visited.add(nb)
                queue.append(nb)
    unreachable = deck0_rooms - visited
    print(f"  Starting from room {start} (corridor)")
    print(f"  Reachable: {len(visited)}/{len(deck0_rooms)} rooms")
    if unreachable:
        for rid in sorted(unreachable):
            r = rooms[rid]
            print(f"  UNREACHABLE: Room {rid} (type={r['type']}, pos=({r['x']},{r['y']}))")
