TODO:
Get switching between textures working


### Questions unanswered:  
Q: what is the best way to manage multiple textures?  
A: seems like its still to use multiple textures (texture0, texture1)  

Q: how do i draw multiple things at the same time without having to re-upload?  
A: same as above - just need to switch between the textures, bind the right webgltexture, update the texture uniform, draw and repeat