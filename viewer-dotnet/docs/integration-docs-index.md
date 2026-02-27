## Índice de documentación de integraciones

Este documento lista los archivos de documentación relacionados con integraciones y describe el contenido mínimo esperado de cada uno.

---

## 1. `docs/env-setup-windows.md`

**Propósito**: Guía paso a paso para preparar el entorno de desarrollo en Windows.

**Contenido mínimo**:

- Verificación de versiones de Python y .NET.
- Comandos de PowerShell para:
  - Crear la carpeta de trabajo del proyecto.
  - Crear la estructura básica de directorios.
  - Crear y activar un entorno virtual de Python.
  - Instalar dependencias principales de IA (Ultralytics, FastAPI, etc.).
- Verificación y uso básico de GPU NVIDIA con `nvidia-smi` y PyTorch.
- Checklist final de validación del entorno.

---

## 2. `backend-yolo/README-backend-yolo.md`

**Propósito**: Documentar el diseño y estructura del servicio YOLOv8 en Python.

**Contenido mínimo**:

- Estructura de carpetas dentro de `backend-yolo/src/yolo_service`.
- Descripción de:
  - `config.py` (configuración y variables de entorno).
  - `models.py` (modelos pydantic para requests/responses).
  - `inference.py` (carga de modelo y ejecución de YOLO).
  - `main.py` (aplicación FastAPI y endpoints).
- Dependencias principales del servicio (FastAPI, Uvicorn, Ultralytics, OpenCV).
- Comandos para arrancar el servicio (`uvicorn`) en PowerShell.
- Próximos pasos para implementación y testing del servicio.

---

## 3. `viewer-dotnet/docs/integration-yolo-service.md`

**Propósito**: Definir el contrato de comunicación HTTP/JSON entre el viewer .NET y el servicio YOLO.

**Contenido mínimo**:

- Configuración general de la API:
  - Host y puerto por defecto.
  - Ruta base y endpoint principal.
- Especificación del endpoint `POST /infer`:
  - Estructura del payload de entrada (frames, opciones de inferencia).
  - Reglas para codificación Base64 de imágenes.
  - Restricciones y recomendaciones de tamaño y frecuencia.
- Especificación de la respuesta de `/infer`:
  - Estructura de detecciones por frame.
  - Significado de las coordenadas de bounding boxes.
- Formato de errores y códigos posibles.
- Requisitos de tiempo de respuesta y objetivos de rendimiento.
- Ejemplo de flujo completo desde el viewer hasta la respuesta de YOLO.

---

## 4. `viewer-dotnet/docs/viewer-structure.md`

**Propósito**: Describir la estructura de la solución .NET para el viewer y el servicio Windows.

**Contenido mínimo**:

- Estructura propuesta de la solución:
  - Proyectos `Viewer.App`, `Viewer.Service`, `Viewer.Shared`.
- Responsabilidades de cada proyecto.
- Diseño del manejo de layouts de grid (2x2, 3x3, 4x4, 4x5, 5x4).
- Diseño de la integración RTSP:
  - Interfaces `IRtspStream` y `IRtspClientFactory` (conceptual).
- Diseño de overlays de detección:
  - `OverlayRenderer` y mapeo de coordenadas.
- Rol de `Viewer.Service` como orquestador:
  - Carga de configuración.
  - Scheduler de capturas y llamadas a YOLO.

---

## 5. `viewer-dotnet/docs/integration-rtsp-capture.md` (pendiente de creación)

**Propósito**: Documentar cómo se conectan las cámaras y cómo se captura video y snapshots vía RTSP.

**Contenido mínimo esperado**:

- Protocolos soportados (RTSP, variantes básicas).
- Requisitos típicos de configuración de cámaras (formato de stream, resolución, FPS).
- Opciones de librerías para RTSP en .NET (por ejemplo, FFmpeg/LibVLC) y criterios de elección.
- Interfaces abstractas en `Viewer.Shared` para desacoplar la librería concreta:
  - Contrato de `IRtspStream` (métodos para iniciar, detener y obtener frames).
  - Patrones para manejo de reconexiones y errores de red.
- Recomendaciones de rendimiento:
  - Resoluciones sugeridas.
  - Frecuencia de captura de snapshots para inferencia.

---

## 6. `viewer-dotnet/docs/integration-windows-service.md` (pendiente de creación)

**Propósito**: Explicar cómo se instala, configura y mantiene el servicio de Windows que orquesta el sistema.

**Contenido mínimo esperado**:

- Descripción del rol de `Viewer.Service`.
- Pasos en PowerShell para:
  - Instalar el servicio.
  - Iniciarlo, detenerlo y desinstalarlo.
- Requisitos de permisos y cuenta de servicio recomendada.
- Integración con logs (ruta de logs, rotación, niveles).
- Uso de `GET /health` del backend YOLO para verificar disponibilidad antes de iniciar la captura masiva.

---

## 7. `docs/architecture-overview.md` (pendiente de creación)

**Propósito**: Proporcionar una vista global de la arquitectura completa del sistema.

**Contenido mínimo esperado**:

- Diagrama de alto nivel (por ejemplo, en formato Mermaid) que muestre:
  - Cámaras IP.
  - Backend YOLO (Python).
  - Viewer .NET (App y Service).
  - Flujos de datos (RTSP, HTTP/JSON, overlays).
- Descripción de cada capa (video, IA, infraestructura).
- Consideraciones de escalabilidad y límites actuales.

---

## 8. Resumen

La documentación de integraciones se organiza en varios archivos especializados para mantener la separación de responsabilidades y facilitar el mantenimiento:

- Preparación del entorno (`env-setup-windows.md`).
- Backend de inferencia (`README-backend-yolo.md`).
- Contratos de comunicación C# ↔ Python (`integration-yolo-service.md`).
- Estructura del viewer y servicio Windows (`viewer-structure.md`).
- Integración RTSP y Windows Service (documentos pendientes con índice definido).
- Visión general de arquitectura (`architecture-overview.md`, pendiente).

Con este índice, cualquier desarrollador puede localizar rápidamente el documento adecuado para cada parte de la integración.

