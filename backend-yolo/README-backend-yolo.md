## Backend YOLOv8 (Python) – Diseño del servicio

Este documento describe la estructura del servicio de inferencia YOLOv8 en Python que se comunicará con el viewer .NET.

Objetivos:

- Exponer un endpoint HTTP local (`/infer`) alineado con `integration-yolo-service.md`.
- Cargar y reutilizar un modelo YOLOv8 para detectar personas.
- Mantener la configuración y las dependencias organizadas y fácilmente desplegables.

---

## 1. Estructura de carpetas propuesta

Dentro de `backend-yolo`:

```text
backend-yolo/
  src/
    yolo_service/
      __init__.py
      main.py            # Punto de entrada FastAPI
      config.py          # Gestión de configuración y variables de entorno
      models.py          # Tipos de datos (pydantic) para requests/responses
      inference.py       # Lógica de carga de modelo y ejecución de YOLO
      logging_utils.py   # Configuración de logging estructurado
  requirements.txt
  README-backend-yolo.md
```

Esta estructura separa:

- API (en `main.py`).
- Configuración (en `config.py`).
- Tipos de datos y contratos (en `models.py`).
- Lógica de inferencia y post-procesado (en `inference.py`).

---

## 2. Dependencias principales

En `requirements.txt` se incluirán, como mínimo:

- `fastapi`
- `uvicorn[standard]`
- `ultralytics`
- `opencv-python`
- `pydantic`

Estas dependencias se instalan dentro del entorno virtual descrito en `docs/env-setup-windows.md`.

---

## 3. Configuración (`config.py`)

Responsabilidades:

- Leer configuración desde variables de entorno o valores por defecto seguros.
- Exponer un objeto de configuración que el resto del módulo pueda usar.

Parámetros típicos:

- `YOLO_MODEL_NAME` (por ejemplo, `"yolov8n.pt"`).
- `YOLO_DEVICE` (`"cuda"` o `"cpu"`, según disponibilidad).
- `SERVICE_HOST` (por defecto `127.0.0.1`).
- `SERVICE_PORT` (por defecto `8001`).

En PowerShell, las variables se pueden definir con:

```powershell
$Env:YOLO_MODEL_NAME = "yolov8n.pt"
$Env:YOLO_DEVICE = "cuda"
```

`config.py` debe:

- Validar que las combinaciones sean coherentes (por ejemplo, avisar si se pide `"cuda"` pero no está disponible).
- Proveer valores por defecto razonables si no hay variables definidas.

---

## 4. Modelos de datos (`models.py`)

Se recomienda usar `pydantic` para definir los esquemas de entrada y salida, alineándolos con el documento `integration-yolo-service.md`.

Modelos principales:

- `BoundingBox`:
  - `x_min`, `y_min`, `x_max`, `y_max` (enteros).
- `Detection`:
  - `id` (cadena).
  - `label` (cadena, por ejemplo `"person"`).
  - `confidence` (float).
  - `bbox` (`BoundingBox`).
- `InferFrameRequest`:
  - `frame_id`, `camera_id`, `cell_id` (cadenas).
  - `width`, `height` (enteros).
  - `image_format` (cadena, por ejemplo `"jpg"`).
  - `image_base64` (cadena).
- `InferOptions`:
  - `confidence_threshold` (float).
  - `iou_threshold` (float).
  - `max_detections_per_frame` (entero).
  - `classes` (lista de cadenas).
- `InferRequest`:
  - `request_id` (cadena).
  - `timestamp_utc` (cadena o `datetime`).
  - `frames` (lista de `InferFrameRequest`).
  - `options` (`InferOptions` opcional).
- `InferFrameResponse`:
  - `frame_id`, `camera_id` (cadenas).
  - `detections` (lista de `Detection`).
- `InferResponse`:
  - `request_id` (cadena).
  - `timestamp_utc` (cadena).
  - `processing_time_ms` (entero).
  - `frames` (lista de `InferFrameResponse`).

También se puede definir un modelo `ErrorResponse` para respuestas de error:

- `ErrorResponse`:
  - `code` (cadena).
  - `message` (cadena).
  - `details` (objeto opcional con información adicional).

---

## 5. Lógica de inferencia (`inference.py`)

Responsabilidades:

- Cargar el modelo YOLOv8 una sola vez al inicio del servicio.
- Proveer una función que reciba un `InferRequest` y devuelva un `InferResponse`.

Pasos a alto nivel:

1. **Carga de modelo**:
   - Usar `ultralytics` para cargar el modelo configurado en `YOLO_MODEL_NAME`.
   - Configurar el dispositivo (`cpu` o `cuda`) según `YOLO_DEVICE` y disponibilidad.
2. **Preprocesado de imágenes**:
   - Decodificar `image_base64` a bytes.
   - Convertir a imagen (por ejemplo con `cv2.imdecode`).
   - Opcionalmente reescalar a una resolución fija o aceptable para el modelo.
3. **Inferencia batch**:
   - Ejecutar el modelo sobre todas las imágenes del request en batch para ganar rendimiento.
4. **Post-procesado**:
   - Filtrar por clase `"person"` (según `options.classes`).
   - Aplicar umbral de confianza e IoU de `options`.
   - Mapear las detecciones a objetos `Detection` con `BoundingBox` en píxeles relativos a la imagen enviada.
5. **Construir respuesta**:
   - Rellenar `InferResponse` con `request_id`, `timestamp_utc` (puede reutilizar el original) y `processing_time_ms`.
   - Asegurar que cada `InferFrameResponse` esté vinculado al `frame_id` y `camera_id` de entrada.

La función principal puede tener una firma similar a:

- `def run_inference(request: InferRequest) -> InferResponse:`

---

## 6. API HTTP con FastAPI (`main.py`)

Responsabilidades:

- Crear la aplicación FastAPI.
- Definir rutas:
  - `GET /health` para chequeo rápido de salud.
  - `POST /infer` como endpoint principal de inferencia.
- Integrar logging y manejo de errores.

Flujo de `/infer`:

1. FastAPI recibe un `InferRequest` (validado por `pydantic`).
2. Registra la llegada de la petición (request id, número de frames, timestamp).
3. Llama a `run_inference`.
4. Devuelve el `InferResponse` como JSON.

`GET /health`:

- Respuesta sencilla indicando que el servicio está listo:

```json
{
  "status": "ok",
  "model_loaded": true
}
```

Esto será útil para que el viewer .NET verifique que el backend de YOLO está disponible antes de enviar peticiones de inferencia.

---

## 7. Logging (`logging_utils.py`)

Para facilitar el diagnóstico:

- Configurar logging estructurado (por ejemplo, formato JSON o texto con campos claros).
- Incluir, como mínimo:
  - Timestamp.
  - Nivel de log.
  - `request_id` cuando exista.
  - Información sobre errores de decodificación de imágenes o fallos de modelo.

Ejemplos de eventos útiles:

- Inicio del servicio y carga de modelo.
- Fallos en la carga del modelo.
- Peticiones de `/infer` con número de frames y tiempo total de procesamiento.
- Errores de decodificación Base64 o formatos de imagen no soportados.

---

## 8. Arranque del servicio

Con el entorno virtual activado y las dependencias instaladas, el servicio se puede arrancar con:

```powershell
Set-Location "C:\Users\Entheus\Desktop\entheus_stream_analitycs\backend-yolo"
.\.venv\Scripts\Activate.ps1
uvicorn yolo_service.main:app --host 127.0.0.1 --port 8001
```

Donde `app` es la instancia de FastAPI creada en `main.py`.

En fases posteriores, se podrá empaquetar este arranque en un script específico o integrarlo como servicio gestionado por el orquestador en C#.

---

## 9. Próximos pasos

- Implementar físicamente los módulos `config.py`, `models.py`, `inference.py` y `main.py` siguiendo este diseño.
- Añadir tests unitarios para la lógica de post-procesado (filtros por clase, umbrales, mapeo de coordenadas).
- Validar manualmente el endpoint `/infer` usando herramientas como `curl` o `Invoke-RestMethod` desde PowerShell antes de integrarlo con el viewer.

