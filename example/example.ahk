; ^ = Ctrl
; # = Win
; ! = Alt
; + = Shift
; Arrows = Left, Right, Up, Down
; Navigation = PgUp, PgDn, Home, End
; Editing = Insert, Delete (or Del), Backspace (or BS)
; System = Esc, Space, Tab, Enter

#Include <FancyTWM>

+#Right:: FancyTWM_Send("MoveToNextVirtualDesktop")
+#Left:: FancyTWM_Send("MoveToPrevVirtualDesktop")

+#1:: FancyTWM_Send("MoveToVirtualDesktop", ["0"])
+#2:: FancyTWM_Send("MoveToVirtualDesktop", ["1"])
+#3:: FancyTWM_Send("MoveToVirtualDesktop", ["2"])
+#4:: FancyTWM_Send("MoveToVirtualDesktop", ["3"])

; ^#Right:: FancyTWM_Send("ChangeToNextVirtualDesktop")
; ^#Left:: FancyTWM_Send("ChangeToPrevVirtualDesktop")

^#1:: FancyTWM_Send("ChangeToVirtualDesktop", ["0"])
^#2:: FancyTWM_Send("ChangeToVirtualDesktop", ["1"])
^#3:: FancyTWM_Send("ChangeToVirtualDesktop", ["2"])
^#4:: FancyTWM_Send("ChangeToVirtualDesktop", ["3"])
