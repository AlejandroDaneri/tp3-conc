Iniciamos la ejecucion del programa con la creacion de un Client al cual le asignamos como id el numero de proceso actual del programa.
Inmediatamente comenzamos la ejecucion del cliente.
Como primer paso creamos un ConnectionHandler que se encarga de avisar a todos los demas clientes su ingreso a la red. Y, a su vez, queda a la espera de nuevos clientes que vayan ingresando a la red. Esto lo hace tomando un puerto que este disponible en el rango de puertos que se le especifican.
Por otro lado, se crea un InputHandler el cual suministra al hilo principal del cliente de los comandos que se escriben por entrada estandar para ser procesados por el cliente  
--Completar PeerHandler cuando ya quede bien definido--

Por ultimo y no menos importante, se llama a una funcion dispatcher la cual se encargar de procesar todos los mensajes que se van recibiendo, tanto desde otro Peer via TCP o los que se ingresaron por entrada estandar.
Hay varios tipos de mensajes:
