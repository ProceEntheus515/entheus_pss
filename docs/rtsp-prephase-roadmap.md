## Hoja de ruta previa a Fase 4 – Validación RTSP

Objetivo: reducir al mínimo la incertidumbre sobre conexión RTSP y autenticación **antes** de invertir en la integración completa de Fase 4.

Esta hoja de ruta es corta, concreta y se centra en verificar que:

- Podemos conectar desde esta máquina a una cámara vía RTSP fuera de SmartPSS.
- Sabemos qué librería .NET usaremos para RTSP.
- Tenemos un POC mínimo en .NET que abre al menos una cámara real.

---

## Desarrollo sin cámaras propias: streams RTSP públicos

Mientras no haya RTSP accesible desde cualquier red (NVR en sitio remoto, P2P sin RTSP expuesto, etc.), se puede **desarrollar usando streams RTSP públicos de prueba**. Así el viewer, el backend YOLO y el POC .NET se validan desde cualquier máquina (casa, trabajo) sin depender del NVR real.

**Ejemplos de URLs RTSP públicas (sin garantía de disponibilidad; probar en VLC):**

- **Wowza (puerto 1935):** `rtsp://807e9439d5ca.entrypoint.cloud.wowza.com:1935/app-rC94792j/068b9c9a_stream2` — ver [Wowza RTSP Stream Test](https://www.wowza.com/developer/rtsp-stream-test/) si la URL cambia.
- **IPVM demo (puerto 5541):** `rtsp://demo:demo@ipvmdemo.dyndns.org:5541/onvif-media/media.amp?profile=profile_1_h264&sessiontimeout=60&streamtype=unicast`
- Servicios tipo “IP Webcam” o “test RTSP” que ofrecen URLs de ejemplo

**Uso:** Probar primero en VLC que la URL abre; luego usar la misma URL en el POC .NET (LibVLCSharp) y en los tests de integración. Cuando se tenga acceso RTSP a las cámaras reales (misma red, VPN o DDNS + port forwarding), se sustituye por las URLs del NVR/cámaras.

**Si los públicos fallan (timeout en 554, DNS en 1935, etc.):** La opción más estable es un **servidor RTSP local** en tu PC. No depende de internet ni de DNS.

### Servidor RTSP local con mediaMTX (recomendado para desarrollo)

[mediaMTX](https://github.com/bluenviron/mediamtx) (antes rtsp-simple-server) permite publicar un vídeo o una fuente como RTSP en localhost.

1. Descargar el binario para Windows desde [releases](https://github.com/bluenviron/mediamtx/releases).
2. Crear un `mediamtx.yml` en la misma carpeta (o usar el de ejemplo) con un path que sirva un archivo de vídeo, por ejemplo:
   ```yaml
   paths:
     test:
       source: file:///ruta/al/video.mp4
       # o source: record
   ```
3. Ejecutar: `mediamtx.exe` (o `./mediamtx`).
4. En VLC abrir: `rtsp://127.0.0.1:8554/test` (puerto por defecto 8554).

Así validas VLC, LibVLCSharp y el flujo YOLO contra un RTSP que siempre responde en tu máquina. Para producción se cambia la URL por la del NVR/cámara real.

---

## Paso 0 – Recopilar datos desde SmartPSS/DSS

- Extraer para al menos **una cámara**:
  - IP o dominio del NVR/cámara.
  - Puerto RTSP (típicamente 554 u otro configurado).
  - Usuario y contraseña que usas en SmartPSS.
  - URL RTSP que SmartPSS o el dispositivo ofrezca, si está visible.
- Documentar estos datos de forma segura (no subir credenciales al repo).

**Nota sobre dispositivos protegidos:** Muchas cámaras y NVR (p. ej. Dahua) desactivan ICMP (ping) por seguridad. Que no responda `ping` o que `Test-NetConnection` marque PingSucceeded = False **no implica IP incorrecta**. La validación real es intentar RTSP (puerto 554) desde la misma red donde SmartPSS ve las cámaras; si VLC o nuestro POC abren el stream, la IP y la URL son válidas. No usar ping como criterio de éxito.

Resultado esperado:

- Una URL RTSP candidata con este formato aproximado:

```text
rtsp://USUARIO:CONTRASEÑA@IP:PUERTO/ruta/del/stream
```

**Credenciales con caracteres especiales:** Si el usuario o la contraseña contienen `#`, `@`, `:`, `/`, `?`, `&` o `%`, deben codificarse en la URL (percent-encoding). Ejemplo: `#` → `%23`, `@` → `%40`. Si no se codifican, el parser de la URL corta en ese carácter y las credenciales se interpretan mal (p. ej. contraseña `Gimenez#515` se lee solo como `Gimenez` y falla la autenticación).

---

## Paso 1 – Validar RTSP desde un cliente estándar

Antes de pensar en .NET:

- Probar la URL RTSP obtenida con:
  - **VLC** (recomendado):
    - Menú “Medio” → “Abrir ubicación de red” → pegar la URL RTSP.
  - Alternativa: `ffplay` si está disponible.

Condiciones para considerar este paso como superado:

- El stream se reproduce correctamente en VLC (o cliente equivalente).
- No hay errores de autenticación ni fallos de conexión.

Si falla:

- Ajustar credenciales, puertos o permisos hasta que **funcione fuera de .NET**.
- Revisar la sección siguiente (requisitos Dahua) y comprobar códec, bitrate y formato de URL.

---

## Requisitos Dahua para RTSP (según wiki oficial)

La documentación de Dahua para *"Acceso remoto/RTSP a través de VLC"* indica lo siguiente. Si **no conecta de ninguna manera**, verificar en el NVR/cámara y en la URL:

**Prerrequisitos (wiki Dahua):**

- La cámara/NVR debe estar en línea y el puerto RTSP abierto (por defecto 554).
- **Códec:** La señal **debe ser H.264**; **no puede ser H.265** para este acceso con VLC según el wiki. Si el stream está en H.265, cambiar en el NVR/cámara a H.264 (encode/video) para la ruta que uses por RTSP.
- **Bitrate:** Velocidad de bits de la transmisión **4096 o inferior**. Si está por encima, bajar en la configuración del canal.
- Se puede usar códec inteligente si está en H.264.

**Variantes de URL (ejemplos del wiki):**

- Con credenciales en la URL:  
  `rtsp://USUARIO:CONTRASEÑA@IP:554/cam/realmonitor?canal=1&subtipo=0`  
  (Algunos firmwares usan `channel`/`subtype` en inglés; si falla, probar `canal`/`subtipo`.)
- Con unicast y ONVIF:  
  `rtsp://IP:554/cam/realmonitor?canal=1&subtipo=0&unicast=verdadero&proto=Onvif`  
  (Probar añadiendo `&unicast=verdadero&proto=Onvif` a la URL que ya usas.)
- Subtype: `0` suele ser flujo principal, `1` subflujo; probar ambos si uno falla.

**Checklist cuando no conecta:**

1. Probar desde un equipo en la **misma red/VLAN** donde SmartPSS ve las cámaras (la IP 192.168.1.245 está bien configurada en el NVR; el fallo suele ser red o firewall).
2. En el NVR/cámara: **encode del canal en H.264** y **bitrate ≤ 4096** para el stream que expones por RTSP.
3. Probar URL con **credenciales** (`rtsp://user:pass@IP:554/...`) y variantes **con y sin** `&unicast=verdadero&proto=Onvif`.
4. Probar **canal** (1, 2, …) y **subtipo** 0 y 1 según corresponda al canal que quieras ver.

**Interpretar logs de VLC (Herramientas → Mensajes):** Si aparece `connection timed out`, `live555 error: Failed to connect` o `access_realrtsp error: cannot connect to IP:554`, el fallo es de **red** (no se llega al dispositivo o el puerto 554 está bloqueado), no de autenticación ni de URL. Si apareciera "401 Unauthorized" o similar, sería credenciales. Para depurar: usar nivel de mensajes 2 o Depuración y reproducir de nuevo.

---

## Paso 2 – Elegir librería RTSP en .NET

Una vez verificado que la URL funciona:

- Seleccionar **una sola** librería de reproducción RTSP en .NET (para no dispersar esfuerzos).
- Candidata principal para este proyecto:
  - **LibVLCSharp** (ecosistema maduro, soporte WPF aceptable en Windows).

Documentar esta decisión en `viewer-dotnet/docs/integration-rtsp-capture.md`:

- Nombre de la librería.
- Motivos de elección.
- Versiones mínimas recomendadas.

---

## Paso 3 – POC mínimo RTSP en .NET (aislado)

Crear un pequeño POC independiente de la UI grande:

- Proyecto simple (WPF o consola) en una carpeta de pruebas (puede ser dentro de `viewer-dotnet` o un sandbox separado).
- El POC debe:
  - Usar LibVLCSharp (u otra librería elegida) para:
    - Abrir la URL RTSP validada en el Paso 1.
    - Renderizar el vídeo en una ventana WPF **o** confirmar recepción de frames si es consola.

Condiciones de éxito:

- La aplicación .NET muestra el vídeo de al menos una cámara real.
- La inicialización de la librería y la conexión RTSP no producen errores repetitivos.

Este POC sirve como “prueba de compatibilidad” entre RTSP + autenticación + red + librería elegida.

---

## Paso 4 – Estrategia de credenciales segura

Antes de integrar RTSP en el viewer principal:

- Decidir cómo manejar credenciales en el entorno final:
  - Variables de entorno.
  - Archivos de configuración externos (`appsettings.json` con secretos fuera del control de versiones).
- Asegurarse de que:
  - Las URLs RTSP en el código **no** contienen credenciales hardcodeadas.
  - `CameraConfig` en `Viewer.Shared` pueda:
    - Guardar solo IDs.
    - Construir URLs a partir de credenciales almacenadas de forma segura (paso posterior).

---

## Paso 5 – Checklist de salida de esta hoja de ruta

Antes de entrar formalmente en la Fase 4 del roadmap principal, verificar:

- [x] Al menos una URL RTSP funciona en VLC u otro cliente estándar (misma máquina).
- [x] Hay una librería .NET elegida y documentada (LibVLCSharp con `LibVLCSharp.WPF` + `VideoLAN.LibVLC.Windows`).
- [x] Existe un POC en .NET que abre un stream RTSP real sin errores críticos (integrado en `Viewer.App` y validado con `VideoDebugWindow`).
- [x] Se ha pensado y documentado la estrategia básica para manejar credenciales de cámaras sin hardcodearlas (construcción de URL en UI, sin persistir credenciales en código fuente).

Cumplidos estos puntos, se considera **cerrada la hoja de ruta previa a Fase 4**, y la integración RTSP en el viewer avanza sobre una base validada.

