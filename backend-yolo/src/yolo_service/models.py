"""
Modelos de datos del servicio YOLO.

Estos modelos definen el contrato de entrada y salida de la API HTTP,
alineados con lo descrito en viewer-dotnet/docs/integration-yolo-service.md.

Se usa pydantic para validar y documentar automáticamente la estructura
de los mensajes que intercambiamos con el viewer en C#.
"""

from __future__ import annotations

from datetime import datetime
from typing import Any, Dict, List, Optional, Union

from pydantic import BaseModel, Field


class BoundingBox(BaseModel):
    x_min: int = Field(..., description="Coordenada X mínima (esquina superior izquierda).")
    y_min: int = Field(..., description="Coordenada Y mínima (esquina superior izquierda).")
    x_max: int = Field(..., description="Coordenada X máxima (esquina inferior derecha).")
    y_max: int = Field(..., description="Coordenada Y máxima (esquina inferior derecha).")


class InferFrameRequest(BaseModel):
    frame_id: str = Field(..., description="Identificador único del frame dentro del viewer.")
    camera_id: str = Field(..., description="Identificador lógico de la cámara.")
    cell_id: str = Field(..., description="Identificador lógico de la celda del grid.")
    width: int = Field(..., description="Ancho del frame en píxeles.")
    height: int = Field(..., description="Alto del frame en píxeles.")
    image_format: str = Field(..., description="Formato de la imagen (por ejemplo, 'jpg').")
    image_base64: str = Field(..., description="Contenido de la imagen codificada en Base64.")


class InferOptions(BaseModel):
    confidence_threshold: float = Field(
        default=0.5,
        description="Confianza mínima para aceptar detecciones.",
    )
    iou_threshold: float = Field(
        default=0.45,
        description="Umbral IoU para la supresión de no-máximos.",
    )
    max_detections_per_frame: int = Field(
        default=50,
        description="Máximo de detecciones por frame.",
    )
    classes: List[str] = Field(
        default_factory=lambda: ["person"],
        description="Lista de clases a filtrar; por defecto solo personas.",
    )


class InferRequest(BaseModel):
    request_id: str = Field(..., description="Identificador único de la petición.")
    timestamp_utc: Union[datetime, str] = Field(
        ...,
        description="Marca de tiempo UTC (ISO 8601) en el momento de la captura aproximada.",
    )
    frames: List[InferFrameRequest] = Field(
        default_factory=list,
        description="Lista de frames a procesar en batch.",
    )
    options: Optional[InferOptions] = Field(
        default=None,
        description="Opciones de inferencia; si es None se aplican valores por defecto.",
    )


class Detection(BaseModel):
    id: str = Field(..., description="Identificador interno de la detección.")
    label: str = Field(..., description="Etiqueta/clase detectada (por ejemplo, 'person').")
    confidence: float = Field(..., description="Confianza de la detección (0 a 1).")
    bbox: BoundingBox = Field(..., description="Caja delimitadora en coordenadas de imagen.")


class InferFrameResponse(BaseModel):
    frame_id: str = Field(..., description="Identificador del frame original.")
    camera_id: str = Field(..., description="Identificador de la cámara original.")
    detections: List[Detection] = Field(
        default_factory=list,
        description="Lista de detecciones para el frame.",
    )


class InferResponse(BaseModel):
    request_id: str = Field(..., description="Identificador de la petición original.")
    timestamp_utc: Union[datetime, str] = Field(
        ...,
        description="Marca de tiempo asociada a la respuesta.",
    )
    processing_time_ms: int = Field(
        ...,
        description="Tiempo total de procesamiento de la petición en milisegundos.",
    )
    frames: List[InferFrameResponse] = Field(
        default_factory=list,
        description="Resultados por frame.",
    )


class ErrorDetail(BaseModel):
    code: str = Field(..., description="Código de error lógico.")
    message: str = Field(..., description="Mensaje descriptivo del error.")
    details: Optional[Dict[str, Any]] = Field(
        default=None,
        description="Información adicional opcional sobre el error.",
    )

