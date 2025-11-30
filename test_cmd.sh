#!/bin/bash
echo "Line 1"
sleep 0.5
echo "Line 2"
sleep 0.5
echo "Line 3 - stderr" >&2
sleep 0.5
echo "Line 4"
echo "Done!"
