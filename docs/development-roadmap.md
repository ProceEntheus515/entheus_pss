## Hoja de ruta de desarrollo

Esta hoja de ruta resume el orden de implementación para que el proyecto avance de forma controlada, sin desviarse del diseño acordado.

Las fases están pensadas para ejecutarse en orden, pero algunas subtareas se pueden paralelizar si es necesario.

---

## Fase 0 – Infraestructura básica y entorno

- Verificar entorno en la máquina de desarrollo siguiendo `env-setup-windows.md`.
- Asegurar que la estructura de carpetas base existe:
  - `backend-yolo/`, `viewer-dotnet/`, `docs/`.
- Revisar `build-and-deploy.md` para entender desde el inicio los objetivos de compilación y despliegue.

**Criterio de salida**: entorno validado y carpetas base creadas.

---

## Fase 1 – Esqueleto del backend YOLO (Python)

- Crear la estructura de `backend-yolo/src/yolo_service` según `README-backend-yolo.md`:
  - `main.py` (FastAPI con `/health` y stub de `/infer`).
  - `config.py` (lectura de variables de entorno, parámetros básicos).
  - `models.py` (modelos pydantic alineados con `integration-yolo-service.md`).
  - `inference.py` (función stub `run_inference` que de momento devuelva detecciones falsas para pruebas).
- Probar localmente con `uvicorn`:
  - `GET /health` debe devolver `status: ok`.
  - `POST /infer` debe aceptar el payload y devolver una estructura válida, aunque las detecciones sean simuladas.

**Criterio de salida**: servicio Python responde correctamente con contratos JSON válidos (sin aún usar YOLO real).

---

## Fase 2 – Esqueleto del viewer .NET

- Crear la solución `ViewerSolution` y los proyectos:
  - `Viewer.App` (WPF/WinUI).
  - `Viewer.Service` (Windows Service o Worker Service).
  - `Viewer.Shared` (modelos y contratos comunes).
- Implementar en `Viewer.Shared`:
  - DTOs para `InferRequest` / `InferResponse` basados en `integration-yolo-service.md`.
  - Modelos de configuración (`CameraConfig`, `LayoutConfig`, `ZoneConfig`).
- En `Viewer.App`:
  - Crear una ventana básica con un grid “falso” (sin RTSP todavía) que represente layouts 2x2, 3x3, etc. con bloques de color.
  - Conectar la lógica de layout (`GridLayoutManager`) para generar las celdas.

**Criterio de salida**: app .NET muestra grids configurables y puede construir un payload de ejemplo para `/infer`.

---

## Fase 3 – Comunicación Viewer ↔ Backend YOLO (sin RTSP real)

- Implementar en `Viewer.Shared` o `Viewer.App` un cliente HTTP:
  - Envía un `InferRequest` de prueba al backend Python.
  - Recibe y valida `InferResponse`.
- Reemplazar los bloques “falsos” del grid por imágenes de prueba (por ejemplo, ficheros locales) para simular cámaras.
- Dibujar overlays de detección básicos en el viewer a partir de la respuesta.

**Criterio de salida**: flujo completo “imagen estática → /infer → detecciones → overlay” funcionando con imágenes de prueba.

---

## Fase 4 – Integración RTSP y captura de frames

- Diseñar y documentar en `integration-rtsp-capture.md` la librería o enfoque RTSP elegido.
- Implementar interfaces en `Viewer.Shared`:
  - `IRtspStream`: iniciar/detener stream, obtener último frame.
  - `IRtspClientFactory`: crear streams a partir de `CameraConfig`.
- Integrar en `Viewer.App`:
  - Reemplazar imágenes estáticas por streams RTSP reales.
  - Exponer un método para obtener snapshots desde cada celda para enviar a `/infer`.

**Criterio de salida**: viewer muestra cámaras reales por RTSP y puede capturar frames desde cada celda.

### Estado Fase 4 (2026-03-02)

- [x] Librería RTSP elegida e integrada: `LibVLCSharp.WPF` + `VideoLAN.LibVLC.Windows` en `Viewer.App`.
- [x] `Viewer.App` reproduce un stream RTSP real en layouts 1x1 y 2x2, manteniendo la conexión al cambiar de layout.
- [x] POC visual validado con `VideoDebugWindow` usando el mismo `MediaPlayer`.
- [ ] API de captura de frame por celda pendiente de implementar (se abordará en la siguiente iteración de Fase 4 / inicio Fase 5).

---

## Fase 5 – YOLOv8 real en el backend

- Completar `inference.py`:
  - Cargar modelo YOLOv8 con `ultralytics`.
  - Implementar preprocesado, inferencia batch y post-procesado para detección de personas.
- Ajustar `config.py` para permitir:
  - Seleccionar modelo (`yolov8n`, `yolov8s`, etc.).
  - Elegir dispositivo (`cpu` / `cuda`).
- Añadir tests básicos de inferencia con imágenes de ejemplo.

**Criterio de salida**: `/infer` usa YOLO real y devuelve detecciones coherentes en tiempos aceptables.

---

## Fase 6 – Lógica de alerta perimetral

- En `Viewer.Shared`:
  - Implementar estructuras para definir zonas de intrusión (polígonos o rectángulos) por cámara/celda.
  - Funciones para comprobar intersecciones entre `BoundingBox` de personas y zonas.
  - Histéresis: número mínimo de frames consecutivos en alerta antes de disparar evento.
- En `Viewer.App`:
  - Visualizar gráficamente las zonas configuradas.
  - Destacar celdas en alerta con colores o bordes específicos.
- Definir formato de logs estructurados (JSON o similar) para eventos de alerta.

**Criterio de salida**: el sistema genera alertas coherentes cuando personas entran en zonas definidas.

---

## Fase 7 – Servicio de Windows y ejecución desatendida

- Seguir `integration-windows-service.md` (cuando esté desarrollado) para:
  - Convertir `Viewer.Service` en servicio de Windows funcional.
  - Definir cómo se arranca y supervisa el backend YOLO (como servicio separado o proceso hijo).
- Validar:
  - Arranque automático al iniciar Windows.
  - Manejo de fallos (reintentos, logs).

**Criterio de salida**: el sistema puede funcionar sin usuario logueado, arrancando como servicios Windows.

---

## Fase 8 – Empaquetado y despliegue

- Aplicar los pasos de `build-and-deploy.md`:
  - Publicar viewer y servicio .NET como self-contained.
  - Empaquetar backend YOLO con PyInstaller.
  - Organizar salida en la estructura de `C:\Program Files\PerimeterGuard\...`.
- Definir e implementar el instalador (script PowerShell o MSI) que:
  - Copie los binarios.
  - Registre los servicios.
  - Cree rutas de config y logs.
- Probar en una máquina “limpia” (sin Python ni .NET).

**Criterio de salida**: instalador funcional que deja el sistema operativo y listo en una PC sin entorno de desarrollo.

---

## Fase 9 – Evolución y overlay sobre otros VMS

Fuera del alcance inmediato, pero previsto:

- Añadir soporte multi-monitor y layouts dinámicos avanzados.
- Diseñar overlay “por encima de SmartPSS/DSS u otros VMS” mediante:
  - Captura de pantalla y calibración de grids.
  - Ventanas transparentes always-on-top con click-through.
- Medir performance y optimizar (batch size, modelo YOLO, frecuencia de inferencia).

Esta fase se abordará cuando el flujo completo con viewer propio y RTSP esté estable.

