"""
Punto de entrada HTTP del servicio YOLO.

Este módulo expone una API FastAPI con:
- GET /health: verificación básica del estado del servicio.
- POST /infer: recepción de peticiones de inferencia y delegación en run_inference.

En esta fase inicial, la inferencia devuelve detecciones simuladas,
pero la estructura de entrada/salida ya es la definitiva para que
el viewer en C# pueda integrarse.
"""

from __future__ import annotations

import logging
from typing import Union

from fastapi import FastAPI
from fastapi.responses import JSONResponse

from .config import get_settings
from .inference import run_inference
from .models import ErrorDetail, InferRequest, InferResponse


logging.basicConfig(
    level=logging.INFO,
    format="%(asctime)s [%(levelname)s] %(name)s - %(message)s",
)
logger = logging.getLogger("yolo_service")

app = FastAPI(title="PerimeterGuard YOLO Service")


@app.get("/health")
async def health() -> dict:
    """
    Endpoint de salud simple.

    De momento solo indica que el proceso está vivo; el campo
    model_loaded se actualizará cuando se cargue YOLO real.
    """

    settings = get_settings()

    return {
        "status": "ok",
        "model_loaded": False,
        "host": settings.service_host,
        "port": settings.service_port,
    }


@app.post("/infer", response_model=InferResponse)
async def infer(request: InferRequest) -> Union[InferResponse, JSONResponse]:
    """
    Endpoint principal de inferencia.

    Recibe un lote de frames y devuelve detecciones simuladas para
    validar el flujo end-to-end con el viewer en C#.
    """

    logger.info(
        "Petición /infer recibida: request_id=%s frames=%d",
        request.request_id,
        len(request.frames),
    )

    try:
        response = run_inference(request)
        return response
    except Exception as exc:  # pylint: disable=broad-except
        # En esta fase se captura la excepción de forma amplia para
        # evitar caídas del servicio; más adelante se podrá refinar.
        logger.exception("Error inesperado en /infer")

        error = ErrorDetail(
            code="INTERNAL_ERROR",
            message="Error interno al procesar la petición de inferencia.",
            details={"exception": str(exc)},
        )

        return JSONResponse(
            status_code=500,
            content={"error": error.dict()},
        )

