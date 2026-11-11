import sys
from PIL import Image
for path in sys.argv[1:]:
    im=Image.open(path).convert('RGB'); W,H=im.size; px=im.load(); base=px[W//2,H-100]
    xs={}
    for x in range(W-100,W):
        ys=[y for y in range(76,H-8) if px[x,y]!=base]
        if ys: xs[x]=(ys[0],ys[-1],len(ys),px[x,ys[len(ys)//2]])
    print(path.split('/')[-1],'bg',base)
    prev=None
    for x,v in xs.items():
        key=(v[0],v[1],v[3])
        if key!=prev: print('  x px',x,'y px',v[0],'-',v[1],'n',v[2],'colour',v[3]); prev=key
    if xs: print('  x range px',min(xs),max(xs))
