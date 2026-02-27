## Integración Viewer .NET ↔ Servicio YOLO (Python)

Este documento define el contrato de comunicación entre:

- El viewer/servicio en **C# (.NET)**.
- El servicio de inferencia **YOLOv8** en **Python**.

La comunicación se basa inicialmente en **HTTP/JSON** sobre `localhost`, pensando en un despliegue en la misma máquina. Más adelante se podría migrar a gRPC si se necesita mayor eficiencia.

---

## 1. Configuración general de la API de YOLO

- **Host por defecto**: `http://127.0.0.1`
- **Puerto por defecto**: `8001` (configurable mediante variable de entorno o archivo de configuración en Python).
- **Ruta base**: sin prefijo adicional (por simplicidad en la primera versión).

Desde C#, la URL completa típica de inferencia será:

```text
http://127.0.0.1:8001/infer
```

---

## 2. Endpoint principal: `POST /infer`

### 2.1. Propósito

Recibir en una sola petición un conjunto de imágenes (una por cámara o por celda activa del grid) y devolver detecciones de personas por cada imagen.

### 2.2. Formato de la petición

- Método: `POST`
- URL: `/infer`
- Cabeceras recomendadas:
  - `Content-Type: application/json`
  - `X-Request-Id: <guid>` (opcional, para trazabilidad cruzada).

Cuerpo (`application/json`):

```json
{
  "request_id": "3b1e0c2f-7b2f-4e8b-9cd3-9c2e0a5a8f10",
  "timestamp_utc": "2026-02-27T15:45:12Z",
  "frames": [
    {
      "frame_id": "viewer-1-cell-0",
      "camera_id": "cam_lobby",
      "cell_id": "grid_2x2_0_0",
      "width": 1280,
      "height": 720,
      "image_format": "jpg",
      "image_base64": "<cadena_base64>"
    },
    {
      "frame_id": "viewer-1-cell-1",
      "camera_id": "cam_parking",
      "cell_id": "grid_2x2_0_1",
      "width": 1280,
      "height": 720,
      "image_format": "jpg",
      "image_base64": "<cadena_base64>"
    }
  ],
  "options": {
    "confidence_threshold": 0.5,
    "iou_threshold": 0.45,
    "max_detections_per_frame": 50,
    "classes": ["person"]
  }
}
```

Notas sobre campos:

- `request_id`: identificador único generado por el viewer para correlacionar logs.
- `timestamp_utc`: marca de tiempo ISO 8601 del momento de captura aproximado.
- `frames`: array de frames independientes.
  - `frame_id`: identificador único del frame dentro del viewer.
  - `camera_id`: identificador lógico de la cámara (coincide con la config del viewer).
  - `cell_id`: identificador lógico de la celda del grid donde se renderiza esta cámara.
  - `width` y `height`: dimensiones del frame enviado al servicio (en píxeles).
  - `image_format`: extensión corta del formato (`jpg`, `png`), se prioriza `jpg` por tamaño.
  - `image_base64`: contenido de la imagen codificada en Base64, sin cabecera de data URI.
- `options`:
  - `confidence_threshold`: confianza mínima para aceptar detecciones.
  - `iou_threshold`: umbral de NMS.
  - `max_detections_per_frame`: límite de detecciones por imagen.
  - `classes`: lista de clases a filtrar; al inicio solo `"person"`.

### 2.3. Consideraciones de tamaño y rendimiento

- Se recomienda que las imágenes ya estén reescaladas por el viewer a una resolución razonable antes de enviarlas (por ejemplo, 640x360 o similar), para reducir ancho de banda y latencia.
- En una primera versión, se acepta que la petición contenga tantas imágenes como celdas visibles en el grid (2x2, 3x3, etc.).
- Si el tamaño total del JSON supera varios MB, será necesario:
  - Reducir resolución o calidad de compresión.
  - Ajustar la frecuencia de inferencia.

---

## 3. Formato de la respuesta de `/infer`

Código de estado:

- `200 OK` cuando la inferencia se realizó correctamente.
- Códigos `4xx/5xx` con un cuerpo de error estructurado en caso de fallo.

Cuerpo `200 OK`:

```json
{
  "request_id": "3b1e0c2f-7b2f-4e8b-9cd3-9c2e0a5a8f10",
  "timestamp_utc": "2026-02-27T15:45:12Z",
  "processing_time_ms": 85,
  "frames": [
    {
      "frame_id": "viewer-1-cell-0",
      "camera_id": "cam_lobby",
      "detections": [
        {
          "id": "det-1",
          "label": "person",
          "confidence": 0.82,
          "bbox": {
            "x_min": 320,
            "y_min": 180,
            "x_max": 420,
            "y_max": 460
          }
        }
      ]
    },
    {
      "frame_id": "viewer-1-cell-1",
      "camera_id": "cam_parking",
      "detections": []
    }
  ]
}
```

Detalles:

- `processing_time_ms`: tiempo total de procesamiento de la petición en el servicio Python.
- `frames`: lista con una entrada por frame recibido.
  - `frame_id` y `camera_id`: deben coincidir con los de la petición.
  - `detections`: lista de detecciones para ese frame.
    - `id`: identificador interno de la detección (puede ser un contador o GUID).
    - `label`: clase detectada, en este caso `"person"`.
    - `confidence`: valor entre 0 y 1.
    - `bbox`: coordenadas del bounding box en píxeles respecto a la imagen enviada:
      - `x_min`, `y_min`: esquina superior izquierda.
      - `x_max`, `y_max`: esquina inferior derecha.

El viewer será responsable de convertir estas coordenadas a las coordenadas reales del control gráfico donde se dibujan los overlays.

---

## 4. Formato de errores

En caso de error, el servicio devolverá una respuesta JSON estructurada:

```json
{
  "error": {
    "code": "INVALID_REQUEST",
    "message": "El campo 'frames' es obligatorio y no se encontró.",
    "details": {
      "field": "frames"
    }
  }
}
```

Ejemplos de códigos de error posibles:

- `INVALID_REQUEST`: petición mal formada (faltan campos, tipos incorrectos).
- `MODEL_NOT_LOADED`: el modelo de YOLO no pudo cargarse.
- `INFERENCE_FAILED`: error en la ejecución de la inferencia.
- `INTERNAL_ERROR`: cualquier otra excepción no controlada.

El viewer debe:

- Registrar en logs el código y mensaje de error.
- Implementar una política de reintentos limitada si se considera necesario.

---

## 5. Requisitos de tiempo de respuesta

Para una buena experiencia en tiempo casi real:

- **Objetivo inicial**:
  - Tiempo total de `/infer` para un batch típico (por ejemplo, 4 a 9 cámaras): **< 150 ms** en condiciones normales con GPU.
- Si no se cumple este objetivo:
  - Reducir resolución de entrada de los frames.
  - Bajar la frecuencia de inferencia (por ejemplo, 5–10 FPS efectivos en lugar de 25–30).
  - Probar un modelo YOLO más ligero (`yolov8n` en lugar de `yolov8s` o superior).

Estos parámetros y objetivos deberán documentarse también en los logs del viewer para analizar el rendimiento en producción.

---

## 6. Seguridad y limitaciones

En la primera versión:

- El servicio YOLO se ejecuta **solo en localhost**, sin autenticación, asumiendo máquina controlada.
- No se exponen endpoints hacia la red externa.

Recomendaciones para futuras versiones:

- Añadir autenticación por token si se expone a otras máquinas.
- Limitar el tamaño máximo de la petición para prevenir abusos.
- Definir una cola de peticiones o rechazar peticiones cuando el servicio esté saturado.

---

## 7. Ejemplo de flujo desde el viewer

1. El viewer captura un frame por cada celda visible del grid.
2. Para cada frame:
   - Se codifica la imagen a JPG.
   - Se convierte a Base64.
   - Se crea el objeto con `frame_id`, `camera_id`, `cell_id`, `width`, `height`.
3. Se construye el JSON de la petición con:
   - `request_id` nuevo.
   - `timestamp_utc` actual.
   - Lista `frames`.
   - Opciones de inferencia (`confidence_threshold`, `classes` con `"person"`).
4. Se envía la petición `POST /infer` a `http://127.0.0.1:8001/infer`.
5. Se recibe la respuesta:
   - Se valida que `request_id` coincida.
   - Se recorren las detecciones por `frame_id`.
   - Se mapean las `bbox` a las coordenadas de la celda correspondiente.
6. El viewer actualiza los overlays (bounding boxes, colores de alerta, etc.) sobre las imágenes en tiempo casi real.

