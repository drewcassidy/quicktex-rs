#!/usr/bin/env bash
# requires CMFT to be installed
# get it here: https://github.com/dariomanesku/cmft

blender -b cubemap.blend -F HDR -o cubemap -f 0
cmft \
--inputFacePosX cubemap0000+X.hdr --inputFaceNegX cubemap0000-X.hdr \
--inputFacePosY cubemap0000+Y.hdr --inputFaceNegY cubemap0000-Y.hdr \
--inputFacePosZ cubemap0000+Z.hdr --inputFaceNegZ cubemap0000-Z.hdr \
--generateMipChain true \
--output0params dds,bgr8,cubemap \
--output0 cubemap \
--output1params ktx,rgb8,cubemap \
--output1 cubemap
rm *.hdr
