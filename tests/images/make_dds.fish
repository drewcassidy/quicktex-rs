#!/usr/bin/env fish

nvassemble -cube cubemap+X.png cubemap-X.png cubemap+Y.png  cubemap-Y.png cubemap+Z.png  cubemap-Z.png -noalpha -o "dds/cubemap.dds"

nvcompress -bc3 drill.png "dds/drill.dds"

for format in rgb bc1 bc4 bc5 lumi
    nvcompress -$format peppers16.png "dds/peppers16 $format.dds"
end