$ErrorActionPreference = 'Stop'
Add-Type -AssemblyName System.Drawing
$root = Split-Path $PSScriptRoot -Parent
$bitmap = [System.Drawing.Bitmap]::new(256,256)
$graphics = [System.Drawing.Graphics]::FromImage($bitmap)
$graphics.SmoothingMode = [System.Drawing.Drawing2D.SmoothingMode]::AntiAlias
$graphics.Clear([System.Drawing.ColorTranslator]::FromHtml('#102D36'))
$teal = [System.Drawing.SolidBrush]::new([System.Drawing.ColorTranslator]::FromHtml('#27ACAD'))
$gold = [System.Drawing.SolidBrush]::new([System.Drawing.ColorTranslator]::FromHtml('#C9A65B'))
$graphics.FillRectangle($teal,35,35,18,186)
$font = [System.Drawing.Font]::new('Segoe UI',132,[System.Drawing.FontStyle]::Bold,[System.Drawing.GraphicsUnit]::Pixel)
$graphics.DrawString('D',$font,[System.Drawing.Brushes]::White,56,30)
$graphics.FillRectangle($gold,67,203,131,9)
$stream = [System.IO.MemoryStream]::new()
$bitmap.Save($stream,[System.Drawing.Imaging.ImageFormat]::Png)
$png = $stream.ToArray()
$file = [System.IO.File]::Create((Join-Path $root 'assets/branding/app.ico'))
$writer = [System.IO.BinaryWriter]::new($file)
$writer.Write([uint16]0); $writer.Write([uint16]1); $writer.Write([uint16]1)
$writer.Write([byte]0); $writer.Write([byte]0); $writer.Write([byte]0); $writer.Write([byte]0)
$writer.Write([uint16]1); $writer.Write([uint16]32); $writer.Write([uint32]$png.Length); $writer.Write([uint32]22); $writer.Write($png)
$writer.Dispose(); $graphics.Dispose(); $bitmap.Dispose(); $font.Dispose(); $teal.Dispose(); $gold.Dispose(); $stream.Dispose()
