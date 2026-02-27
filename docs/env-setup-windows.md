## Objetivo

Este documento describe, paso a paso y sin asumir nada, cómo preparar un entorno de desarrollo en Windows para:

- Servicio de inferencia de detección de personas con YOLOv8 en Python.
- Cliente/servicio en C# (.NET) que actuará como viewer de cámaras y orquestador.

Todos los ejemplos de comandos están pensados para ejecutarse en **PowerShell**.

---

## 1. Verificar versiones instaladas actualmente

### 1.1. Verificar versión de Python

En PowerShell:

```powershell
python --version
```

Deberías ver algo similar a:

```text
Python 3.11.0
```

Si no aparece Python o la versión es muy antigua (< 3.10), instalar o actualizar Python desde la página oficial (`https://www.python.org/downloads/windows/`), marcando la opción de **“Add Python to PATH”** durante la instalación.

### 1.2. Verificar .NET SDK

En PowerShell:

```powershell
dotnet --info
```

Debería aparecer información del SDK similar a:

```text
SDK DE .NET:
 Version:           9.0.201
 ...
```

Si no está instalado, descargar el **.NET SDK (versión LTS o actual)** desde `https://dotnet.microsoft.com/es-es/download` e instalarlo.

### 1.3. Verificar GPU NVIDIA y herramientas

En PowerShell:

```powershell
nvidia-smi
```

Si aparece una tabla con la GPU (por ejemplo, `Quadro RTX 3000`), la tarjeta es visible para CUDA. Si el comando no existe o falla, instalar o actualizar:

- Drivers NVIDIA desde la página oficial.
- Paquete **CUDA Toolkit** compatible con tu versión de drivers y sistema operativo (`https://developer.nvidia.com/cuda-downloads`).

La configuración fina de CUDA/cuDNN se hará más adelante, una vez que el proyecto de Python esté creado.

---

## 2. Crear carpeta de trabajo del proyecto

Supondremos que el proyecto vive en:

```text
C:\Users\Entheus\Desktop\entheus_stream_analitycs
```

Si la carpeta no existe, crearla:

```powershell
New-Item -ItemType Directory -Path "C:\Users\Entheus\Desktop\entheus_stream_analitycs" -Force | Out-Null
Set-Location "C:\Users\Entheus\Desktop\entheus_stream_analitycs"
```

Estructura inicial de carpetas (puede crearse con PowerShell o usando el explorador):

```powershell
New-Item -ItemType Directory -Path ".\docs" -Force | Out-Null
New-Item -ItemType Directory -Path ".\backend-yolo\src" -Force | Out-Null
New-Item -ItemType Directory -Path ".\viewer-dotnet\src" -Force | Out-Null
New-Item -ItemType Directory -Path ".\viewer-dotnet\docs" -Force | Out-Null
```

---

## 3. Preparar entorno de Python para YOLOv8

### 3.1. Crear entorno virtual

Es recomendable aislar las dependencias en un entorno virtual dentro de `backend-yolo`:

```powershell
Set-Location "C:\Users\Entheus\Desktop\entheus_stream_analitycs\backend-yolo"
python -m venv .venv
```

Activar el entorno virtual:

```powershell
.\.venv\Scripts\Activate.ps1
```

Cuando el entorno está activo, el prompt de PowerShell suele mostrar el prefijo `(.venv)` al principio de la línea.

Para desactivar el entorno:

```powershell
deactivate
```

### 3.2. Actualizar `pip` y herramientas básicas

Con el entorno virtual activo:

```powershell
python -m pip install --upgrade pip wheel setuptools
```

### 3.3. Instalar Ultralytics YOLOv8 y dependencias mínimas

Con el entorno virtual activo:

```powershell
pip install ultralytics fastapi uvicorn[standard] opencv-python
```

Notas:

- `ultralytics` proporciona YOLOv8 y utilidades asociadas.
- `fastapi` y `uvicorn` se usarán para exponer un servicio HTTP local de inferencia.
- `opencv-python` se utilizará para manejo de imágenes y posibles transformaciones.

### 3.4. (Opcional) Aceleración por GPU con CUDA

Si se desea aprovechar la GPU NVIDIA, es necesario que la instalación de PyTorch que viene con `ultralytics` reconozca CUDA.

Pasos generales:

1. Verificar versión de CUDA instalada (`nvidia-smi` suele indicar una versión recomendada).
2. Instalar una versión de PyTorch compatible con CUDA de acuerdo con la tabla de compatibilidad de `https://pytorch.org/`.
3. Validar en un intérprete de Python dentro del entorno virtual:

```python
import torch
print(torch.cuda.is_available())
print(torch.cuda.get_device_name(0) if torch.cuda.is_available() else "Sin GPU CUDA disponible")
```

Si `torch.cuda.is_available()` devuelve `True`, la GPU está lista para ser usada por YOLOv8.

---

## 4. Preparar entorno .NET para el viewer y el servicio Windows

### 4.1. Verificar que `dotnet` funciona en PowerShell

En cualquier PowerShell:

```powershell
dotnet --info
```

Si el comando responde correctamente, el SDK está listo para usar. En caso contrario, instalar el SDK desde la página oficial y reiniciar PowerShell.

### 4.2. Crear solución y proyectos base

Desde la carpeta raíz del workspace:

```powershell
Set-Location "C:\Users\Entheus\Desktop\entheus_stream_analitycs\viewer-dotnet"
dotnet new sln -n ViewerSolution
```

La creación de los proyectos concretos (aplicación WPF/WinUI, servicio Windows y biblioteca compartida) se detallará en un documento específico de estructura del viewer, para mantener la separación de responsabilidades.

---

## 5. Variables de entorno y credenciales

Para cumplir con el principio de no hardcodear credenciales:

- Usuarios, contraseñas y direcciones sensibles de cámaras deben ir en **variables de entorno** o en archivos de configuración externos protegidos.

Ejemplos en PowerShell (sesión actual):

```powershell
$Env:CAMERA_1_RTSP = "rtsp://usuario:password@ip:puerto/ruta"
```

Para definir variables de entorno de forma persistente a nivel de usuario:

```powershell
setx CAMERA_1_RTSP "rtsp://usuario:password@ip:puerto/ruta"
```

Estas variables podrán ser leídas tanto desde Python como desde C#.

---

## 6. Checklist de validación rápida del entorno

Antes de continuar con la implementación, verificar:

1. **Python**:
   - `python --version` devuelve 3.10 o superior.
   - El entorno virtual `.venv` se activa sin errores.
   - `python -c "import ultralytics, fastapi, cv2"` se ejecuta sin excepciones dentro del entorno.
2. **GPU (opcional pero recomendado)**:
   - `nvidia-smi` funciona y muestra la GPU.
   - Dentro de Python: `torch.cuda.is_available()` devuelve `True` si se configuró CUDA.
3. **.NET**:
   - `dotnet --info` muestra el SDK instalado.
   - Desde `viewer-dotnet`, se puede ejecutar `dotnet new sln -n ViewerSolution` sin errores.
4. **Estructura de carpetas**:
   - Existe la raíz `C:\Users\Entheus\Desktop\entheus_stream_analitycs`.
   - Existen las carpetas `docs`, `backend-yolo\src`, `viewer-dotnet\src`, `viewer-dotnet\docs`.

Si alguno de estos pasos falla, documentar el error exacto del comando y solucionarlo antes de seguir con el desarrollo de las integraciones.

