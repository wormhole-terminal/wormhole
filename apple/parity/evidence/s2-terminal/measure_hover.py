import sys
from PIL import Image
def m(path):
    im=Image.open(path).convert('RGB'); W,H=im.size; px=im.load()
    base=px[2000,400]
    ys=[y for y in range(60,1500) if px[2000,y]!=base]
    tint=px[2000,(ys[0]+ys[-1])//2]
    y=(ys[0]+ys[-1])//2
    xs=[x for x in range(W) if px[x,y]==tint]
    pts=[(x,yy) for yy in range(1400,1630) for x in range(2000,2630) if sum(abs(a-b) for a,b in zip(px[x,yy],base))>20]
    X=[p[0] for p in pts]; Y=[p[1] for p in pts]
    bb=(min(X),min(Y),max(X),max(Y))
    fill=px[bb[0]+6,bb[1]+6]
    # green/red exit text: most saturated pixel in pill
    sat=max(((max(px[x,yy])-min(px[x,yy]),px[x,yy],x) for yy in range(bb[1],bb[3]) for x in range(bb[0],bb[2])))
    # icon bbox: pixels in right 80px of pill that differ from fill
    ic=[(x,yy) for yy in range(bb[1],bb[3]) for x in range(bb[2]-90,bb[2]) if sum(abs(a-b) for a,b in zip(px[x,yy],fill))>40]
    IX=[p[0] for p in ic]; IY=[p[1] for p in ic]
    print(path.split('/')[-1],'base',base,'tint',tint,'tint rows y',ys[0],ys[-1],'x',xs[0],xs[-1],'| pill px',bb,'size',bb[2]-bb[0]+1,bb[3]-bb[1]+1,'fill',fill,'exit-text',sat[1],'icon bbox',(min(IX),min(IY),max(IX),max(IY)))
for p in sys.argv[1:]: m(p)
