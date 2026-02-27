## Estructura del viewer .NET para grids y captura RTSP

Este documento describe cómo organizaremos la solución .NET para:

- Mostrar cámaras IP en grids configurables (2x2, 3x3, 4x4, 4x5, 5x4).
- Capturar frames periódicos para enviar al servicio YOLO.
- Dibujar overlays de detección y alerta perimetral.

La prioridad es mantener responsabilidades claras, microcomponetización y una base sólida para futuras extensiones.

---

## 1. Estructura de solución propuesta

Dentro de `viewer-dotnet`:

- `ViewerSolution.sln` (solución principal).
- Proyectos:
  - `Viewer.App` (aplicación WPF/WinUI de escritorio para el grid y overlays).
  - `Viewer.Service` (Windows Service para orquestación en background).
  - `Viewer.Shared` (lógica compartida, DTOs, contratos y utilidades).

Organización general:

```text
viewer-dotnet/
  src/
    Viewer.App/
    Viewer.Service/
    Viewer.Shared/
  docs/
    viewer-structure.md
    integration-yolo-service.md
```

La solución se puede crear con:

```powershell
Set-Location "C:\Users\Entheus\Desktop\entheus_stream_analitycs\viewer-dotnet"
dotnet new sln -n ViewerSolution
```

La creación de cada proyecto (`dotnet new`) se recomienda hacer siguiendo este diseño, pero se implementará en una fase posterior.

---

## 2. Proyecto `Viewer.Shared`

Responsabilidades:

- Definir modelos y contratos reutilizables por `Viewer.App` y `Viewer.Service`:
  - DTOs para petición/respuesta al servicio YOLO (alineados con `integration-yolo-service.md`).
  - Modelos de configuración (cámaras, layouts, zonas perimetrales).
  - Lógica común de validación y utilidades.

Subcarpetas sugeridas:

- `Configuration/`
  - `CameraConfig.cs`
  - `LayoutConfig.cs`
  - `ZoneConfig.cs`
  - `AppSettings.cs`
- `Detection/`
  - `DetectionResult.cs`
  - `BoundingBox.cs`
- `Contracts/`
  - `YoloInferRequest.cs`
  - `YoloInferResponse.cs`
- `Services/`
  - Interfaces para abstracciones (por ejemplo, `IYoloClient`, `IFrameCaptureService`).

Esta separación facilita pruebas unitarias y evita duplicación de modelos.

---

## 3. Proyecto `Viewer.App` (WPF/WinUI)

Responsabilidades:

- Mostrar las cámaras en un grid configurable.
- Gestionar layouts (2x2, 3x3, 4x4, 4x5, 5x4).
- Dibujar overlays de detecciones y estados de alerta sobre el video.
- En futuro, servir como interfaz de configuración avanzada.

### 3.1. Layouts de grid

En el nivel de UI se utilizará un contenedor que permita:

- Definir número de filas y columnas dinámicamente.
- Asociar cada celda a una `CameraConfig`.

Diseño conceptual de clases:

- `GridLayoutManager`:
  - Entrada: lista de cámaras activas y tipo de layout deseado (por ejemplo, `"2x2"`, `"3x3"`).
  - Salida: mapa `GridCell` con coordenadas lógicas (fila, columna) y referencia a una cámara.
- `GridCell`:
  - Propiedades: `RowIndex`, `ColumnIndex`, `CameraId`, `CellId`.

El viewer usará este mapa para saber:

- Qué stream RTSP pertenece a cada celda.
- Dónde dibujar las detecciones que llegan desde el servicio YOLO.

### 3.2. Captura RTSP

La reproducción y captura de frames desde RTSP se realizará mediante una librería externa (por ejemplo, bindings de FFmpeg o LibVLC para .NET). El diseño debe:

- Encapsular la dependencia en una interfaz:
  - `IRtspStream` (iniciar, detener, obtener frames).
  - `IRtspClientFactory` (crear instancias de streams a partir de `CameraConfig`).
- Permitir:
  - Renderizar el video en los controles propios de WPF/WinUI.
  - Obtener snapshots (frames individuales) a demanda o de forma periódica.

Cada celda del grid tendrá asociado un objeto de tipo `IRtspStream` responsable de:

- Mantener la conexión con la cámara.
- Proveer el último frame disponible para enviar al servicio YOLO cuando el scheduler lo solicite.

### 3.3. Overlays de detección

Para cada celda del grid:

- Se mantiene una lista de detecciones activas recibidas del servicio YOLO.
- Se transforman las coordenadas de `BoundingBox` (en resolución de imagen) a coordenadas de control gráfico.
- Se dibujan cajas, textos y cambios de color según el estado de alerta.

Diseño conceptual:

- `OverlayRenderer`:
  - Entrada: tamaño actual del control de video y lista de `DetectionResult`.
  - Función: convertir a elementos visuales (rectángulos, etiquetas) que WPF/WinUI pueda mostrar encima del video.

---

## 4. Proyecto `Viewer.Service` (Windows Service)

Responsabilidades:

- Ejecutar el sistema de forma desatendida como servicio de Windows.
- Gestionar la carga y validación de configuración.
- Arrancar y supervisar:
  - El viewer (si se usa en modo sin UI o con UI opcional).
  - El cliente YOLO para inferencia.
- Gestionar logs, reintentos y manejo de errores global.

Conceptualmente, este servicio:

- Lee `appsettings.json` o archivo equivalente de configuración.
- Inicializa estructuras en `Viewer.Shared` (cámaras, layouts, zonas).
- Configura el scheduler que decide cada cuánto se envían frames a YOLO.
- Expone, si es necesario, un canal de administración (por ejemplo, un endpoint HTTP local para estadísticas o control).

La integración específica con el sistema de servicios de Windows se documentará en `integration-windows-service.md`.

---

## 5. Flujo de datos dentro del viewer

Resumen del flujo principal:

1. `Viewer.Service` carga la configuración y construye modelos compartidos (`CameraConfig`, `LayoutConfig`, `ZoneConfig`).
2. `Viewer.App` (o lógica visual equivalente) recibe esa configuración y:
   - Crea el layout de grid mediante `GridLayoutManager`.
   - Abre un `IRtspStream` por cada cámara activa.
3. Un scheduler en `Viewer.Shared` o en `Viewer.Service`:
   - Cada X milisegundos solicita el último frame disponible de cada `IRtspStream`.
   - Genera una petición `YoloInferRequest` y la envía al servicio Python.
4. Se recibe un `YoloInferResponse`:
   - Se mapean detecciones por `frame_id` y `camera_id`.
   - Se actualizan estructuras de detecciones por celda.
5. `Viewer.App` usa `OverlayRenderer` para dibujar overlays de detección y destacar celdas en alerta.

---

## 6. Consideraciones de extensibilidad

El diseño propuesto permite:

- Cambiar la librería de RTSP sin tocar la lógica de detección ni layouts, gracias a las interfaces de `IRtspStream`.
- Ajustar la resolución y la frecuencia de captura desde la configuración.
- Reemplazar el backend de YOLO (por ejemplo, usar ONNX Runtime en C#) reutilizando los modelos y contratos ya definidos en `Viewer.Shared`.

Este documento sirve como guía para crear los proyectos .NET y organizar el código de forma coherente con los objetivos de detección perimetral y mantenimiento a largo plazo.

