## Uso futuro de Rust en el sistema de detección perimetral

Este documento NO define trabajo inmediato, sino posibles escenarios futuros en los que tendría sentido incorporar Rust en el proyecto, partiendo de la arquitectura actual Python + C#.

---

## Contexto actual

- **Backend de IA**: Python (FastAPI + Uvicorn + Ultralytics YOLOv8).
- **Viewer y servicio Windows**: C# (.NET, WPF + Worker Service).
- **Comunicación**: HTTP/JSON local (`/infer`) con contratos ya definidos y alineados.

Con esta arquitectura cubrimos bien las necesidades actuales de:

- Detección de personas por cámara.
- Overlay en grids de cámaras.
- Servicio Windows instalable en máquinas sin entorno de desarrollo.

Rust solo se considerará cuando haya cuellos de botella concretos que lo justifiquen.

---

## Escenario 1 – Ingesta y preprocesado intensivo de vídeo

### Cuándo tendría sentido

- Muchas cámaras RTSP (decenas o más) a altas resoluciones y FPS.
- Necesidad de:
  - Reescalar frames.
  - Convertir formatos de color.
  - Recortar múltiples regiones de interés por frame.
  - Hacer todo eso con latencia muy baja y uso de CPU controlado.

### Posible rol de Rust

- Crear un módulo/servicio `video-ingest` en Rust que:
  - Decodifique streams RTSP.
  - Aplique el preprocesado pesado.
  - Entregue frames ya preparados (bytes, tensores o imágenes comprimidas) a Python o C#.

### Integración con el sistema actual

- Opción A: biblioteca Rust expuesta a Python (FFI) o C# (P/Invoke) como módulo nativo.
- Opción B: microservicio adicional (por ejemplo, gRPC), donde:
  - C# pide frames procesados a Rust.
  - Python recibe ya datos optimizados para inferencia.

---

## Escenario 2 – Tracking avanzado y análisis multi-cámara

### Cuándo tendría sentido

- Evolución de simple detección de personas a:
  - Tracking persistente (asignar IDs a individuos).
  - Seguimiento entre múltiples cámaras.
  - Cálculo de trayectorias, permanencias, conteos avanzados.

### Posible rol de Rust

- Implementar un motor de tracking/analítica en Rust que:
  - Consuma las detecciones de YOLO (cajas y labels).
  - Mantenga estructuras de datos grandes y complejas en memoria.
  - Efectúe cálculos intensivos de forma más predecible y eficiente que Python.

### Integración con el sistema actual

- Python seguiría entregando detecciones brutas.
- Rust recibiría:
  - Listas de detecciones por frame y cámara.
  - Información temporal.
- Devuelve:
  - Eventos de alto nivel (persona X en zona Y durante Z segundos, conteos acumulados, trayectorias).

---

## Escenario 3 – Overlay de muy baja latencia sobre SmartPSS/DSS u otros VMS

### Cuándo tendría sentido

- Cuando se requiera:
  - Overlay directo sobre ventanas de terceros (SmartPSS, DSS, otros VMS).
  - Captura de pantalla o composición gráfica de muy baja latencia.
  - Manipulación intensiva de píxeles, con integración fuerte con APIs gráficas de Windows.

### Posible rol de Rust

- Módulo/servicio Rust que:
  - Capture regiones de pantalla a alta frecuencia.
  - Aplique compositing de overlays (cajas, zonas, indicadores) usando APIs de bajo nivel (GDI+, DirectX, etc.).
  - Trabaje en sincronía con la UI de C#, recibiendo solo datos de detección (no toda la lógica de negocio).

### Integración con el sistema actual

- C# seguiría orquestando:
  - Configuración, grids, reglas de alerta.
  - Comunicación con Python (YOLO).
- Rust se usaría como acelerador gráfico específico para overlays complejos u overlays por encima de VMS externos.

---

## Escenario 4 – Despliegues en edge / hardware muy limitado

### Cuándo tendría sentido

- Cuando el sistema se quiera ejecutar en:
  - Gateways pequeños.
  - NVRs personalizados.
  - Dispositivos ARM con poca RAM/CPU.

### Posible rol de Rust

- Escribir componentes críticos en Rust para:
  - Reducir tamaño de binarios.
  - Controlar al máximo el uso de memoria.
  - Minimizar dependencias pesadas de runtimes.

En este escenario, se evaluaría:

- Sustituir partes del backend Python por servicios Rust.
- O encapsular toda la lógica de preprocesado/tracking en Rust, manteniendo YOLO donde sea más razonable (Python o C++).

---

## Principios para decidir introducir Rust

Antes de añadir Rust al proyecto, se recomienda:

1. **Medir**:
   - Identificar claramente dónde están los cuellos de botella (CPU, GPU, disco, red, UI).
2. **Validar alternativas en el stack actual**:
   - Optimizar Python (batching, tamaños de imagen, uso de GPU).
   - Optimizar C# (mejor gestión de hilos, asincronía, buffers).
3. **Acotar el alcance**:
   - Introducir Rust solo en módulos pequeños, con interfaces bien definidas.
   - Mantener contratos claros (por ejemplo, sobre HTTP/gRPC o FFI bien tipado).
4. **Documentar la integración**:
   - Cada módulo Rust debería tener su propio `.md` de integración, igual que el resto del sistema.

Solo cuando estos pasos indiquen que Rust aporta una mejora significativa y justificada en coste/beneficio, se propondrá un diseño concreto de módulo y su plan de migración.

