#!/bin/bash

export
while read line ; do
#for line in `seq 0 10`; do
	if [ $(($RANDOM % 2)) == "0" ] ; then
		echo "STDOUT: $line"
	else
		echo "STDERR: $line" 1>&2
	fi
sleep 1
done
