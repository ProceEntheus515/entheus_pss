"""
Configuración central del servicio YOLO.

Este módulo define una única fuente de verdad para parámetros como:
- host y puerto del servicio HTTP local.
- nombre del modelo YOLO a utilizar en fases posteriores.
- dispositivo de ejecución (cpu / cuda).

Los valores se leen desde variables de entorno cuando existen, y se
les aplican valores por defecto seguros cuando no están definidas.
"""

from functools import lru_cache
import os


class Settings:
    """
    Clase sencilla de configuración basada en variables de entorno.

    Se evita depender de pydantic-settings para mantener el número
    de dependencias bajo control en esta fase inicial.
    """

    def __init__(self) -> None:
        self.service_host: str = os.getenv("YOLO_SERVICE_HOST", "127.0.0.1")

        try:
            self.service_port: int = int(os.getenv("YOLO_SERVICE_PORT", "8001"))
        except ValueError:
            self.service_port = 8001

        self.yolo_model_name: str = os.getenv("YOLO_MODEL_NAME", "yolov8n.pt")
        self.yolo_device: str = os.getenv("YOLO_DEVICE", "cpu")


@lru_cache(maxsize=1)
def get_settings() -> Settings:
    """
    Devuelve una instancia única de Settings.

    Se usa un caché para evitar recrear la configuración en cada
    petición y garantizar un punto único de lectura de entorno.
    """

    return Settings()  # type: ignore[arg-type]

