## Viewer .NET – Inicio rápido de la interfaz

Este documento explica cómo arrancar la interfaz inicial del viewer en C# para ver los grids de cámaras de prueba y generar el payload de `/infer`.

---

## 1. Requisitos previos

- Haber seguido la Fase 0 del roadmap y tener:
  - .NET SDK instalado (verificable con `dotnet --info`).
  - El repositorio en `C:\Users\Entheus\Desktop\entheus_stream_analitycs`.
- (Opcional, pero recomendado para pruebas completas) Tener el backend YOLO corriendo según `backend-yolo/README-backend-yolo.md`, aunque para esta interfaz inicial no es obligatorio.

---

## 2. Estructura relevante

Dentro de `viewer-dotnet`:

- `ViewerSolution.sln` – solución principal.
- `src/Viewer.App/` – aplicación WPF (`PerimeterGuard Viewer`).
- `src/Viewer.Shared/` – modelos compartidos (layouts, cámaras, DTOs YOLO).
- `src/Viewer.Service/` – esqueleto del servicio de fondo (no necesario para esta prueba).

---

## 3. Cómo arrancar la interfaz WPF

1. Abrir una consola **PowerShell**.
2. Posicionarse en la raíz del workspace:

```powershell
Set-Location "C:\Users\Entheus\Desktop\entheus_stream_analitycs"
```

3. Lanzar la aplicación WPF:

```powershell
dotnet run --project "viewer-dotnet/src/Viewer.App/Viewer.App.csproj"
```

4. Debería abrirse una ventana titulada **Entheus Stream Analitycs Viewer**.

---

## 4. Qué muestra la interfaz en esta fase

La ventana actual es una vista de prueba con:

- Un **selector de layout** en la parte superior:
  - Opciones iniciales: `2x2` y `3x3`.
- Un **grid de celdas de color** en la parte central:
  - Cada celda representa una cámara lógica.
  - El texto de cada celda indica:
    - `CellId` (por ejemplo, `2x2_0_0`).
    - `CameraId` asignado (por ejemplo, `cam_1`, `cam_2`, etc.).
- Un botón **“Generar JSON de /infer”**:
  - Construye un `InferRequestDto` de ejemplo para todas las celdas visibles.
  - Muestra el JSON resultante en un cuadro de diálogo (`MessageBox`).

Cambiar entre `2x2` y `3x3` en el combo vuelve a generar el grid y el mapeo de celdas.

---

## 5. Siguiente paso después de esta interfaz

Esta interfaz sirve como base visual para:

- Ver cómo se organizan las celdas por layout.
- Ver el **payload JSON** que se enviará al backend YOLO (`POST /infer`).

En fases posteriores se añadirán:

- Carga de imágenes reales (primero estáticas, luego RTSP).
- Llamadas HTTP reales al backend YOLO desde C#.
- Dibujo de overlays de detección sobre las “cámaras”.

