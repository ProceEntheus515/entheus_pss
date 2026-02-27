"""
Lógica de inferencia del servicio YOLO.

En esta fase inicial (Fase 1) la función run_inference no ejecuta un
modelo YOLO real. Su propósito es:
- Validar el flujo de datos extremo a extremo.
- Devolver una respuesta con la estructura correcta.

Más adelante esta implementación se reemplazará por una que cargue
un modelo YOLOv8 y ejecute inferencias reales sobre los frames.
"""

from __future__ import annotations

from datetime import datetime
from time import perf_counter
from typing import List

from .models import (
    BoundingBox,
    Detection,
    InferFrameRequest,
    InferFrameResponse,
    InferRequest,
    InferResponse,
)


def _build_dummy_detections(frame: InferFrameRequest) -> List[Detection]:
    """
    Genera detecciones simuladas para un frame.

    Esta función existe solo para pruebas iniciales del contrato JSON.
    Se devuelve, como ejemplo, una única caja centrada en la imagen.
    """

    if frame.width <= 0 or frame.height <= 0:
        return []

    box_width = max(frame.width // 4, 1)
    box_height = max(frame.height // 3, 1)

    x_min = max((frame.width - box_width) // 2, 0)
    y_min = max((frame.height - box_height) // 2, 0)
    x_max = min(x_min + box_width, frame.width)
    y_max = min(y_min + box_height, frame.height)

    bbox = BoundingBox(x_min=x_min, y_min=y_min, x_max=x_max, y_max=y_max)

    detection = Detection(
        id=f"dummy-{frame.frame_id}",
        label="person",
        confidence=0.9,
        bbox=bbox,
    )

    return [detection]


def run_inference(request: InferRequest) -> InferResponse:
    """
    Ejecuta la \"inferencia\" sobre los frames de la petición.

    En esta fase devuelve detecciones simuladas, pero respeta
    exactamente la estructura esperada en la respuesta para que
    el viewer en C# pueda integrarse sin depender todavía de YOLO real.
    """

    start = perf_counter()

    frames_responses: List[InferFrameResponse] = []

    for frame in request.frames:
        detections = _build_dummy_detections(frame)

        frame_response = InferFrameResponse(
            frame_id=frame.frame_id,
            camera_id=frame.camera_id,
            detections=detections,
        )
        frames_responses.append(frame_response)

    elapsed_ms = int((perf_counter() - start) * 1000)

    response = InferResponse(
        request_id=request.request_id,
        timestamp_utc=request.timestamp_utc or datetime.utcnow().isoformat(),
        processing_time_ms=elapsed_ms,
        frames=frames_responses,
    )

    return response

