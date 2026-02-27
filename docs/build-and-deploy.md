## Contrato de compilación y despliegue

Este documento define cómo empaquetar el sistema para que se pueda instalar y ejecutar en una PC **sin entorno de desarrollo** (sin Python, sin SDK .NET, sin VS).

El objetivo es producir binarios y, opcionalmente, un instalador que:

- Instale el **viewer .NET** y el **backend YOLO** como programas listos para usar.
- Configure los **servicios de Windows** necesarios.
- Utilice solo dependencias de sistema razonables (por ejemplo, drivers NVIDIA para GPU).

---

## 1. Artefactos de build esperados

Se generarán dos artefactos principales:

1. **Viewer .NET (C#)**:
   - Publicado como aplicación **self-contained** para `win-x64`.
   - No requiere que .NET esté instalado en la máquina destino.
2. **Backend YOLO (Python)**:
   - Empaquetado como ejecutable Windows (por ejemplo, con **PyInstaller**).
   - No requiere instalación previa de Python en la máquina destino.

Opcionalmente, un tercer artefacto:

3. **Instalador** (MSI/EXE):
   - Copia ambos artefactos en las rutas de programa.
   - Registra uno o varios servicios de Windows.
   - Crea carpetas de configuración y logs.

---

## 2. Contrato de compilación – Viewer .NET

### 2.1. Requisitos previos en la máquina de build

- SDK .NET instalado (verificado con `dotnet --info`).
- Código fuente organizado según `viewer-structure.md`.

### 2.2. Comando de publicación recomendado

Desde `viewer-dotnet`:

```powershell
Set-Location "C:\Users\Entheus\Desktop\entheus_stream_analitycs\viewer-dotnet"
dotnet restore

dotnet publish .\src\View er.App\Viewer.App.csproj `
  -c Release `
  -r win-x64 `
  --self-contained true `
  -p:PublishSingleFile=true `
  -p:IncludeNativeLibrariesForSelfExtract=true `
  -o .\publish\viewer-app
```

Notas:

- `--self-contained true`: incluye el runtime de .NET, no hace falta que esté instalado en la máquina destino.
- `PublishSingleFile=true`: genera un único ejecutable grande, más fácil de distribuir.
- `IncludeNativeLibrariesForSelfExtract=true`: garantiza la inclusión de bibliotecas nativas necesarias.

Si el servicio de Windows vive en otro proyecto (por ejemplo `Viewer.Service`), se puede publicar de forma similar:

```powershell
dotnet publish .\src\Viewer.Service\Viewer.Service.csproj `
  -c Release `
  -r win-x64 `
  --self-contained true `
  -p:PublishSingleFile=true `
  -p:IncludeNativeLibrariesForSelfExtract=true `
  -o .\publish\viewer-service
```

### 2.3. Estructura de salida esperada

Después de publicar, se espera algo similar a:

```text
viewer-dotnet/
  publish/
    viewer-app/
      Viewer.App.exe
      (archivos auxiliares si los hubiera)
    viewer-service/
      Viewer.Service.exe
      (archivos auxiliares si los hubiera)
```

Estos ejecutables son los que se copiarán a la máquina destino.

---

## 3. Contrato de compilación – Backend YOLO (Python)

### 3.1. Requisitos previos en la máquina de build

- Python 3.10+ instalado.
- Entorno virtual de `backend-yolo` configurado según `env-setup-windows.md`.
- Dependencias instaladas (`ultralytics`, `fastapi`, `uvicorn`, `opencv-python`, etc.).
- `pyinstaller` instalado en el entorno virtual:

```powershell
Set-Location "C:\Users\Entheus\Desktop\entheus_stream_analitycs\backend-yolo"
.\.venv\Scripts\Activate.ps1
pip install pyinstaller
```

### 3.2. Comando de empaquetado con PyInstaller

Asumiendo que el punto de entrada del servicio es `yolo_service/main.py` con una variable `app` de FastAPI:

```powershell
Set-Location "C:\Users\Entheus\Desktop\entheus_stream_analitycs\backend-yolo"
.\.venv\Scripts\Activate.ps1

pyinstaller `
  --onefile `
  --name yolo-backend `
  --add-data "path\al\modelo\yolov8n.pt;." `
  .\src\yolo_service\main.py
```

Notas:

- `--onefile`: genera un único ejecutable `yolo-backend.exe`.
- `--add-data`: se usa para incluir el archivo del modelo YOLO dentro del ejecutable o en la misma carpeta. La ruta concreta del modelo se ajustará cuando esté definida.

### 3.3. Estructura de salida esperada

PyInstaller creará varias carpetas, pero lo importante es:

```text
backend-yolo/
  dist/
    yolo-backend.exe
```

`yolo-backend.exe` es el binario que se copiará a la máquina destino y se registrará como servicio de Windows o se lanzará desde el servicio .NET.

---

## 4. Estructura unificada en la máquina destino

Se recomienda usar una ruta estándar, por ejemplo:

```text
C:\Program Files\PerimeterGuard\
  viewer\
    Viewer.App.exe
    Viewer.Service.exe
  backend\
    yolo-backend.exe
  config\
    appsettings.json
    cameras.json
    zones.json
  logs\
    viewer\
    backend\
```

Características:

- `viewer\`: ejecutables del viewer y del servicio Windows.
- `backend\`: ejecutable del backend YOLO.
- `config\`: archivos de configuración editables por el administrador.
- `logs\`: registros de actividad y errores.

Esta estructura debe ser respetada tanto por el proceso de instalación como por los binarios en tiempo de ejecución.

---

## 5. Registro de servicios de Windows

Existen varias opciones para registrar servicios:

- Usar el propio soporte de servicios de Windows en C# (`Worker Service` o `Windows Service`).
- Usar herramientas de línea de comandos como `sc.exe` o `New-Service` en PowerShell.

Ejemplo conceptual con PowerShell para registrar el servicio del viewer:

```powershell
$serviceName = "PerimeterGuard.Viewer"
$serviceExe = "C:\Program Files\PerimeterGuard\viewer\View er.Service.exe"

New-Service -Name $serviceName -BinaryPathName "`"$serviceExe`"" -DisplayName "Perimeter Guard Viewer Service" -StartupType Automatic
```

Para el backend YOLO, se podría:

- Registrar `yolo-backend.exe` como servicio de Windows por separado.
- O lanzar `yolo-backend.exe` desde el servicio .NET (gestionando su ciclo de vida).

La estrategia exacta se documentará con más detalle en `integration-windows-service.md`.

---

## 6. Instalador (MSI/EXE) – Requisitos mínimos

Aunque la implementación concreta del instalador puede variar (WiX, Advanced Installer, Inno Setup, script PowerShell), el contrato mínimo que debe cumplir es:

1. **Copiar archivos**:
   - Copiar `Viewer.App.exe`, `Viewer.Service.exe` y `yolo-backend.exe` a `C:\Program Files\PerimeterGuard\...`.
   - Crear subcarpetas `config` y `logs`.
   - Opcionalmente, colocar archivos de configuración de ejemplo.
2. **Registrar servicios**:
   - Crear al menos el servicio `PerimeterGuard.Viewer`.
   - Opcionalmente, crear un servicio `PerimeterGuard.YoloBackend` si se decide que el backend sea un servicio independiente.
3. **Configurar permisos**:
   - Asegurar que la cuenta de servicio tiene permisos para:
     - Leer `config\`.
     - Escribir en `logs\`.
4. **Desinstalación limpia**:
   - Detener y eliminar los servicios.
   - Eliminar archivos de programa (respetando, si se decide, los archivos de configuración y logs).

---

## 7. Validación post-instalación

Después de instalar en una máquina limpia, se debe verificar:

1. Los servicios aparecen en la lista de servicios de Windows y arrancan sin errores.
2. El viewer puede leer la configuración de cámaras y mostrar los grids.
3. El backend YOLO responde en `http://127.0.0.1:8001/health` y acepta peticiones `/infer`.
4. Los logs se generan correctamente en `C:\Program Files\PerimeterGuard\logs\...`.

Esta validación puede documentarse más adelante como una checklist separada, pero forma parte del contrato de despliegue exitoso.

