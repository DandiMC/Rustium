# Mueve el raton de forma relativa (dx, dy) sobre la ventana que tenga el foco.
# Los dos juegos capturan el cursor y leen el movimiento relativo, asi que con
# esto se puede girar la camara sin tocar el teclado.
#   dy negativo = mirar hacia arriba, dy positivo = hacia abajo
param(
    [int]$dx = 0,
    [int]$dy = 0,
    [int]$veces = 1,
    [switch]$click
)

Add-Type -Namespace MR -Name U -MemberDefinition @'
[DllImport("user32.dll")]
public static extern void mouse_event(uint dwFlags, int dx, int dy, uint dwData, int dwExtraInfo);
'@

if ($click) {
    [MR.U]::mouse_event(0x0002, 0, 0, 0, 0)   # LEFTDOWN (solo agarra el cursor)
    [MR.U]::mouse_event(0x0004, 0, 0, 0, 0)   # LEFTUP
    Start-Sleep -Milliseconds 300
}

for ($i = 0; $i -lt $veces; $i++) {
    [MR.U]::mouse_event(0x0001, $dx, $dy, 0, 0)   # MOUSEEVENTF_MOVE
    Start-Sleep -Milliseconds 30
}
