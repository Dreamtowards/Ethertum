

int MaxIdx(float3 v) {
    float a=v.x;
    float b=v.y;
    float c=v.z;
    return a > b ? (a > c ? 0 : 2) : (b > c ? 1 : 2);
}

float mod(float v, float n)
{
	float f = v % n;
	return f < 0 ? f + n : f;
	//return v-n*floor(v/n);
}
	
float4 TexTrip(UnityTexture2D tex, float3 p, float MtlTexId, float3 weights, float MtlTexCap) 
{
	float E = 0.02 / MtlTexCap;  // intoduce Epsilon to fix Mipmap Error (and Float-point Error) on Tex Boundary
	float MtlTexSizeX = 1.0 / MtlTexCap;
	float MtlTexPosX  = MtlTexId / MtlTexCap;	
	float2 uvX = float2(mod(p.z * MtlTexSizeX, MtlTexSizeX-E*2) + MtlTexPosX+E, p.y);
	float2 uvY = float2(mod(p.x * MtlTexSizeX, MtlTexSizeX-E*2) + MtlTexPosX+E, p.z);
	float2 uvZ = float2(mod(p.x * MtlTexSizeX, MtlTexSizeX-E*2) + MtlTexPosX+E, p.y);

	//SAMPLE_TEXTURE2D(tex, TexSampleState, uvX) * weights.x +
    return tex2D(tex, uvX) * weights.x +
           tex2D(tex, uvY) * weights.y +
           tex2D(tex, uvZ) * weights.z;
}

void MtlBlend_float(
	float3 MtlIds,
	float3 WorldPos,
	float3 WorldNorm,
	float3 BaryCoord,
	UnityTexture2D TexDiff,
	UnityTexture2D TexNorm,
	UnityTexture2D TexDRAM,
	UnitySamplerState TexSampleState,
	float MtlTexScale,
	float MtlTexCap,
	float MtlBlendPowTriplanar,
	float MtlBlendPowHeightmap,
	float MtlTexIdOffset,

	out float3 oAlbedo,
	out float3 oNormal, 
	out float oMetallic,
	out float oSmoothness,
	out float3 oEmission,
	out float oAO)
{
	int i_MaxBary = MaxIdx(BaryCoord);
	//int iax_MaxNorm = MaxIdx(WorldNorm);
	
    // use Norm AbsVal as weights
    float3 BlendTrip = pow(abs(WorldNorm), MtlBlendPowTriplanar);
    BlendTrip /= dot(BlendTrip, 1.0);  // make sure the weights sum up to 1 (divide by sum of x+y+z)

	MtlIds += MtlTexIdOffset;

	float3 PosTrip = WorldPos / MtlTexScale;

	float4 DRAMv[3] = {
		TexTrip(TexDRAM, PosTrip, MtlIds[0], BlendTrip, MtlTexCap),
		TexTrip(TexDRAM, PosTrip, MtlIds[1], BlendTrip, MtlTexCap),
		TexTrip(TexDRAM, PosTrip, MtlIds[2], BlendTrip, MtlTexCap),
	};
	
	float3 _bhm = pow(BaryCoord, MtlBlendPowHeightmap);  // BlendHeightmap. Pow: littler=mix, greater=distinct, opt 0.3 - 0.6, 0.48 = nature
	int i_MaxHigh = MaxIdx(float3(DRAMv[0].x * _bhm.x, DRAMv[1].x * _bhm.y, DRAMv[2].x * _bhm.z));

	int i_MtlVtx = i_MaxHigh;  // triangle vertex idx of Current Frag Mtl. usually = i_MaxBary, or i_MaxHigh

	float4 DRAM = DRAMv[i_MaxHigh];

	oAlbedo = 
	TexTrip(TexDiff, PosTrip, MtlIds[i_MaxHigh], BlendTrip, MtlTexCap);
					 
	oNormal =
	TexTrip(TexNorm, PosTrip, MtlIds[i_MaxHigh], BlendTrip, MtlTexCap);

	
	oEmission = 0;
	oSmoothness = 1.0 - DRAM.y;
	oAO = DRAM.z;
	oMetallic = 0;
}