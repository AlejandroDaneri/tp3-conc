APP=target/debug/blockchain

declare -A process_map

function launch_process() {
	$APP < input_$1.fifo > output_$1.txt &
	sleep $2
	new_pid=$!
	echo "Launched $1: $new_pid"
	process_map[$1]=$new_pid
}

cargo build --no-default-features

mkfifo input_a.fifo
mkfifo input_b.fifo

{
sleep 3
echo "rb"
} > input_a.fifo &

{
sleep 2
echo "wb insert p 2"
} > input_b.fifo &

launch_process a 0
launch_process b 1

multitail -F blockchain.multitail.conf -CS blockchain -s 2 output_a.txt output_b.txt

echo "killing blockchain processes"
killall blockchain
rm -vf *.fifo