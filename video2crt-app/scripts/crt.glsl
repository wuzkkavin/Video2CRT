//!HOOK MAIN
//!BIND HOOKED
vec4 hook(){vec4 c=HOOKED_tex(HOOKED_pos);float x=HOOKED_pos.x*HOOKED_size.x;int col=int(mod(x,3.0));float strength=0.10;vec3 tint=vec3(0.0);if(col==0)tint=vec3(strength,-strength*0.3,-strength*0.3);else if(col==1)tint=vec3(-strength*0.3,strength,-strength*0.3);else tint=vec3(-strength*0.3,-strength*0.3,strength);float y=HOOKED_pos.y*HOOKED_size.y;int row=int(mod(y,4.0));float vfade=(row<2)?1.0:0.75;return vec4((c.rgb+tint)*vfade,c.a);}
