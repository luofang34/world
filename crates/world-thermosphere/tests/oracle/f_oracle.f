C     Evaluates one NRLMSISE-00 case, read from standard input, with the
C     reference Fortran implementation. The first field is 0 for GTD7
C     and 1 for GTD7D.
      PROGRAM ORACLE
      DIMENSION D(9),T(2),SW(25),AP(7)
      INTEGER ISW(23),IHIST,MODE
      READ(*,*) MODE,IYD,SEC,ALT,GLAT,GLONG,STL,F107A,F107,IHIST,
     &  (AP(I),I=1,7),(ISW(I),I=1,23)
      DO 10 I=1,23
        SW(I)=ISW(I)
   10 CONTINUE
      SW(24)=1.
      SW(25)=1.
      IF(IHIST.EQ.1.AND.ISW(9).EQ.1) SW(9)=-1.
      CALL TSELEC(SW)
      IF(MODE.EQ.1) THEN
        CALL GTD7D(IYD,SEC,ALT,GLAT,GLONG,STL,F107A,F107,AP,48,D,T)
      ELSE
        CALL GTD7(IYD,SEC,ALT,GLAT,GLONG,STL,F107A,F107,AP,48,D,T)
      ENDIF
      WRITE(*,100) (D(I),I=1,9),T(1),T(2)
  100 FORMAT(11ES26.16E3)
      END
