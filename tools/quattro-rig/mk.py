#!/usr/bin/env python3
"""mk.py OUT 'script' - tokens: text is typed; {f2} {enter} {esc} {tab} {up} {down} {left} {right} {home} {end} {del} {bs}; {w5} waits 5s; {d0.5} sets default gap."""
import re,struct,sys
SH='~!@#$%^&*()_+{}|:"<>?'; UN=r"`1234567890-=[]\;',./"
ROW={c:s for s,cs in {2:'1234567890-=',16:'qwertyuiop[]',30:"asdfghjkl;'`",44:'zxcvbnm,./'}.items() for s,c in [(s+i,c) for i,c in enumerate(cs)] for c in [c]}
ROW['\\']=43;ROW[' ']=57
NAM={'enter':(13,28),'esc':(27,1),'tab':(9,15),'bs':(8,14),'up':(0,72),'down':(0,80),'left':(0,75),'right':(0,77),'home':(0,71),'end':(0,79),'del':(0,83),'pgup':(0,73),'pgdn':(0,81)}
for i in range(1,11): NAM[f'f{i}']=(0,58+i)
def tick(s): return int(round(s*18.2065))
out=b'';gap=tick(1.0);pend=gap
def add(a,s):
    global out,pend
    out+=struct.pack('<HBB',pend,a,s);pend=gap
for t in re.findall(r'\{[^}]*\}|.',sys.argv[2],re.S):
    if t[0]=='{' and len(t)>1:
        n=t[1:-1]
        if re.fullmatch(r'w[\d.]+',n): pend+=tick(float(n[1:]))
        elif re.fullmatch(r'd[\d.]+',n): gap=tick(float(n[1:]));pend=gap
        else: add(*NAM[n])
    else:
        c=t
        if c in SH: c=UN[SH.index(c)] if c not in '{}' else c
        low=c.lower()
        add(ord(t),ROW[low])
open(sys.argv[1],'wb').write(out)
