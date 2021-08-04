# TP3 - Blockchain Rústica

## Integrantes

- Daneri, Alejandro
- Lafroce, Matias

## Introducción

El presente trabajo práctico tiene como objetivo implementar una funcionalidad de _blockchain_ simplificada.

Para la realización de este trabajo, optamos por realizarlo utilizando los siguientes algoritmos

- **Bully** ( Algoritmo de elección de líder )
- **Algoritmo Centralizado** (Algoritmo de exclusión mutua )

A su vez se ha utilizado el protocolo TCP para la comunicación entre nodos de la red.

## Descubrimiento de nodos activos en la red

Se ha implementado un algoritmo en el cual un nodo que entra a la red puede detectar los demas nodos que se encuentran actualmente en la red de la siguiente manera:

- En primer lugar, el nodo recorrerá todos los puertos disponibles que haya en la red y por cada puerto que se encuentre ocupado se creara un Peer el cual modelara al nodo remoto con el cual se esta conectando. Todas las interacciones posteriores con cualquiera de los nodos remotos se hará por medio de un PeerHandler que se detallara en profundidad más adelante.
- Luego realizar el barrido de puertos se quedara escuchando en un puerto que este disponible a la espera de mensajes de conexión nueva de nuevos nodos que se conecten. Cuando aparece un nuevo nodo se procederá a la creación de otro Peer de la misma manera que se menciono anteriormente

## Algoritmo de elección de líder - Bully

Para la parte de la elección de un nodo _líder_ o _coordinador_, decidimos implementarlo utilizando un algoritmo **Bully**, en donde la elección del coordinador se basa en un criterio simple de que el nodo con el numero de proceso mayor en la red es quien será el _coordinador_.
Dicho algoritmo se ejecuta ante la presencia de los siguiente eventos:

- SI NO HAY NINGÚN OTRO EVENTO AL FINAL CORREGIR ACÁ
- Cuando un nodo detecta que el coordinador actual "esta caído". Si un nodo, al enviarle algún mensaje al líder o coordinador, no recibe respuesta en un tiempo predeterminado, entonces considerará al mismo como desconectado y se procederá a la elección del líder nuevamente.

### Implementación

1. El nodo que comienza la elección envía, solamente a los Peers con numero de proceso mayor que el propio, un mensaje del tipo `LeaderMessage::LeaderElectionRequest` por la red, indicando el comienzo del proceso de elección de líder.
2. Cuando cada nodo reciba un mensaje de este tipo deberá responder al emisor con un mensaje `LeaderMessage::OkMessage`, y repitiendo el paso anterior con sus superiores.
3. Si un determinado nodo no obtiene respuesta alguna de los nodos a los que se contacto en el paso (1), entonces, sera el nuevo líder, por lo que deberá enviar un mensaje `LeaderMessage::CoordinatorMessage` hacia todos los nodos de la red indicando que es el nuevo líder.

Cuando un proceso de líder se encuentre en curso, no podrán seguirse procesando los eventos que necesiten la figura de líder en su procesamiento. Una vez la elección de líder haya finalizado, se podrá seguir con el flujo normal del programa.

## Algoritmo de Exclusión Mutua - Algoritmo Centralizado

### Implementación

## Modo de uso

Para iniciar el proceso y conectar un nuevo nodo a la red bastará con ejecutar
`cargo run`

Una vez que se haya iniciado el proceso, el mismo podrá recibir los siguiente comandos:

```
wb <nombre> <nota> : Solicita agregar la nota de <nombre> con valor <nota> a la blockchain.
rb : Solicita el estado de la blockchain actual.
```

## Estructuras y Traits utilizados

## Conclusión:
