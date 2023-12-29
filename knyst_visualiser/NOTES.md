# Calculate from output

Take all the edges to the graph output and set the position of the source nodes based on the graph output position. Then add any edges from those source nodes to the next level.

The vertical distance between nodes depends on how many inputs they have and how many inputs their inputs have. This could be done iteratively only in the y direction.

What happens if one node is the input of many other nodes? It should go in the furthest column.


1. Do a DFS to find the leaf nodes furthest away from the graph.
2. Start with the leaf node that's the furthest away and work towards the graph outputs.
3. Any nodes not connected to a graph output are placed at the same level as the graph output.

1. Start with the inputs to the graph output. Place those nodes right before.
2. Continue with any nodes that are inputs to the nodes moved in the previous column. If a node is hit twice, it gets moved back which is what we want.