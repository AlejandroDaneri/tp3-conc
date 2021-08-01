## Checklist

- [X] Continuar con la serialización y deserialización de mensajes
- [ ] Crear un hilo que contanga la lista de peers, que atienda eventos para incorporar los peers que vienen con una ClientConnection y para enviar mensajes a los peers
- [ ] Crear un hilo que corrar el process messages. Lo que está en la linea 127 tiene que ser derivado a este hilo.
La respuesta de process_message son encoladas al hilo de los peers
- [X] Extraer de ClientMessages los mensajes de lider, para poder trabajarlos de forma especial
- [X] Crear un hilo para los mensajes de lider.
- [ ] Mutar el lider que apunta el cliente en el thread de lider?.
- [ ] Crear una condvar para trabar el hilo de procesar mensajes. y que el hilo de leaderMessages lo destrabe cuando se elije un lider.

# Blockchain

```
cargo run
```

## Leer blockchain

```
rb
```

## Agregar un dato

```
wb insert Pedro 10
```
