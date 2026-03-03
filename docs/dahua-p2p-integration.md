## Integración Dahua P2P (dh-p2p) como traductor P2P a RTSP

Esta guía define la arquitectura del servicio auxiliar basado en el binario Rust `dh-p2p` que traducirá cámaras Dahua P2P a RTSP local, para que el sistema de analítica y el viewer .NET sigan viendo únicamente URLs RTSP.

---

## Estado de validación (actualizado 2026-03-03)

### Resumen de pruebas ejecutadas

Dispositivo de prueba: NVR Dahua en `192.168.1.252:37777`, serial `8F00218PAG08A4C`, firmware `6.6.5`.

| Fase | Resultado | Notas |
|------|-----------|-------|
| Descubrimiento P2P (Easy4IPCloud) | OK | Serial localizado, servidores DS/US/relay obtenidos |
| Negociacion relay (Agent + Token + SID) | OK | Relay negociado correctamente en ambas implementaciones (Python y Rust) |
| Tunel PTCP via relay (`--relay`) | OK | Sesion PTCP establecida, `Ready to connect!` mostrado |
| Autenticacion RTSP a traves del tunel | OK | `OPTIONS` con Digest auth devolvio `RTSP/1.0 200 OK` |
| Streaming sostenido via relay | FALLO | Relay corta la conexion durante `DESCRIBE` (~30s limite de sesion relay) |
| Conexion directa P2P (sin `--relay`) | FALLO | `ConnectionReset` (os error 10054) al enviar paquete directo a `PubAddr` |

### Diagnostico

- El relay de Dahua (`Time: 30`) no esta disenado para streaming sostenido; su proposito es facilitar NAT traversal.
- La conexion directa P2P falla porque el dispositivo rechaza el paquete UDP inicial hacia `PubAddr`.
- La autenticacion RTSP funciona correctamente a traves del tunel (credenciales `admin` / `fidei515519` aceptadas por la camara).

### Siguiente paso tecnico

Investigar por que la conexion directa P2P (`PubAddr: 181.90.202.36:60903`) rechaza el handshake con `ConnectionReset` y ajustar `src/dh.rs` para que el tunel directo funcione con este modelo de NVR/firmware.

---

## Arquitectura propuesta (sidecar Rust dh-p2p)

- **Objetivo**: ocultar la complejidad del protocolo P2P/PTCP detras de un proceso Rust que expone RTSP local.
- **Patron**: una instancia del binario `dh-p2p.exe` por camara o por NVR, cada una escuchando en un puerto local distinto.

Flujo conceptual:

```mermaid
flowchart LR
  subgraph clientSite [SiteCliente]
    camP2P[CamaraDahuaP2P]
    subgraph p2pLayer [CapaRustP2P]
      dhP2P[dh-p2p.exe_relay]
      rtspLocal[RTSPLocal_1554]
    end
    analytics[AnaliticaPrincipal/.NETViewer]
  end

  camP2P <-- P2P_PTCP_UDP --> dhP2P
  dhP2P --> rtspLocal
  analytics -->|RTSP_uniforme| rtspLocal
```

### Responsabilidades del sidecar Rust

- Resolver el serial P2P de la camara a traves de Easy4IPCloud.
- Establecer el tunel PTCP con el dispositivo remoto (modo relay o directo).
- Levantar un listener TCP local (por defecto `127.0.0.1:1554`) que acepte conexiones RTSP.
- Encapsular/descapsular trafico RTSP entre el cliente local y la camara remota P2P.
- Mantener el tunel vivo con heartbeats PTCP.

### Responsabilidades del sistema de analitica / viewer .NET

- Tratar todas las fuentes de video como RTSP, sin conocer si detras hay:
  - Camara con RTSP nativo.
  - Camara Dahua/KBVision accesible solo via P2P + `dh-p2p`.
- Resolver, a nivel de configuracion, que URL RTSP corresponde a cada camara logica.

---

## Comando estandar para levantar el tunel

```powershell
# Modo relay (funciona con el dispositivo actual, limitado a ~30s)
.\dh-p2p.exe --relay -p 127.0.0.1:1554:554 8F00218PAG08A4C

# Modo directo (objetivo final, requiere ajustes en src/dh.rs)
.\dh-p2p.exe -p 127.0.0.1:1554:554 8F00218PAG08A4C
```

URL RTSP resultante que usara el viewer/analitica:

```text
rtsp://admin:fidei515519@127.0.0.1:1554/cam/realmonitor?channel=1&subtype=0
```

---

## Configuracion propuesta de camaras P2P

Ejemplo de configuracion conceptual (no se guarda en el repo con credenciales reales):

```json
{
  "cameras": [
    {
      "id": "entrada_principal",
      "type": "dahua_p2p",
      "serial": "XXXXXXXXXXXXXXX",
      "username_env": "CAM_ENTRADA_USER",
      "password_env": "CAM_ENTRADA_PASS",
      "local_rtsp_port": 1554
    }
  ]
}
```

Reglas:

- Las credenciales reales se leen desde variables de entorno (`CAM_ENTRADA_USER`, `CAM_ENTRADA_PASS`).
- El servicio expone, para cada camara P2P:
  - `rtsp://USER:PASS@127.0.0.1:{local_rtsp_port}/cam/realmonitor?channel=1&subtype=0`
- El viewer .NET solo ve la URL RTSP local.
- Para multi-camara: cada instancia de `dh-p2p.exe` usa un puerto local distinto (1554, 1555, 1556, ...).

---

## Flujo de conexion para una camara Dahua P2P

1. Se lanza `dh-p2p.exe --relay -p 127.0.0.1:{puerto}:554 {SERIAL}`.
2. El binario:
   - Contacta Easy4IPCloud con el serial.
   - Negocia relay (Agent + Token + SID).
   - Establece sesion PTCP con el dispositivo.
   - Escucha en `127.0.0.1:{puerto}`.
3. El viewer .NET / ffplay / VLC se conecta a la URL RTSP local.
4. El trafico RTSP fluye encapsulado en PTCP sobre UDP a traves de la nube Dahua.

---

## Notas especificas para tu entorno

- Dispositivo de prueba: NVR Dahua, serial `8F00218PAG08A4C`, firmware `6.6.5`.
- Direccion local del NVR: `192.168.1.252:37777` (puerto propietario, no RTSP).
- El binario `dh-p2p` no conecta por IP:puerto directamente, sino mediante serial P2P y la nube.
- Credenciales: `admin` / `fidei515519` (almacenar en variables de entorno, nunca en el repo).
- Policy reportada por el dispositivo: `p2p,udprelay` (soporta ambos modos).

---

## Integracion con el roadmap RTSP existente

Esta capa P2P encaja con la hoja de ruta `rtsp-prephase-roadmap.md` de la siguiente forma:

- Para camaras RTSP nativas:
  - Se usa directamente la URL RTSP del dispositivo o NVR.
- Para camaras solo P2P:
  - Se usa la URL RTSP local generada por el sidecar Rust `dh-p2p`.
- El viewer .NET y el backend de analitica:
  - No necesitan cambios de arquitectura; solo reciben una URL RTSP uniforme.

---

## Proximo trabajo pendiente

1. **Hacer funcionar la conexion directa P2P** (sin `--relay`):
   - Investigar por que `PubAddr` rechaza el paquete inicial con `ConnectionReset`.
   - Hipotesis: el handshake directo necesita autenticacion de canal (como hace Python con `DevAuth`/`IpEncrptV2`) que la implementacion Rust no incluye.
   - Ajustar `src/dh.rs` para portar la logica de autenticacion de canal desde `helpers.py`.
2. **Endurecer el modo relay** como fallback:
   - Manejar reconexion automatica cuando el relay corta a los ~30s.
   - Ciclo de reconexion transparente para el cliente RTSP.
3. **Integrar como servicio**:
   - Script PowerShell o servicio Windows que lance una instancia por camara P2P.
   - Healthchecks y logs estructurados.

