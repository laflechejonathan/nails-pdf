## TODO 

Content Streams are what actually represents rendering instructions in PDF, they look like this:

```
ET
Q
q 0 0 0 rg
BT
318.8 197.8 Td /F2 11 Tf[<32>79<06>-1<13>]TJ
ET
Q
q 0 0 0 rg
BT
389.6 197.8 Td /F2 11 Tf[<2D01>-4<0A>18<06>-1<14>-11<07>-2<08>]TJ
ET
Q
q 0 0 0 rg
BT
389.6 179.1 Td /F3 11 Tf[<23>-2<32>1<25>1<3A>2<06>-2<07>-4<02>4<130B0C132D>4<39>1<2F>-4<07>-4<01>7<3B>-1<1220>34<01>-1<1E>-1<1E>7<0A>-4<35>-4<1E>7<2F>-4<07>-4<030B>]TJ
ET
Q
q 0 0 0 rg
```

They are pretty easy to parse, mostly whitespace separated tokens, we just need to exhaustively list all the different operators, how many operands etc. etc.
