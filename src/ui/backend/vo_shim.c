#include <errno.h>
#include <stdint.h>
#include <stdlib.h>
#include <string.h>
#include <unistd.h>

#include "sample_comm.h"

#define LINTX_VPSS_CHN 0
#define LINTX_VO_CHN 0
#define LINTX_STAGE_FRAME_COUNT 3

typedef struct lintx_stage_frame_s {
	VIDEO_FRAME_INFO_S frame;
	CVI_U8 *virt;
} lintx_stage_frame_t;

typedef struct lintx_vo_ctx_s {
	SAMPLE_VO_CONFIG_S vo_config;
	SIZE_S logical_size;
	SIZE_S panel_size;
	SIZE_S output_size;
	VPSS_GRP vpss_grp;
	VB_POOL stage_pool;
	VB_POOL vo_pool;
	lintx_stage_frame_t stage_frames[LINTX_STAGE_FRAME_COUNT];
	VIDEO_FRAME_INFO_S vo_frame;
	CVI_U8 *vo_virt_base;
	CVI_U8 *vo_virt_y;
	CVI_U8 *vo_virt_uv;
	CVI_U32 stage_blk_size;
	CVI_U32 stage_stride;
	CVI_U32 stage_map_len;
	CVI_U32 vo_blk_size;
	CVI_U32 vo_map_len;
	CVI_U32 vo_stride_y;
	CVI_U32 vo_stride_uv;
	CVI_U32 vo_len_y;
	CVI_U32 vo_len_uv;
	RECT_S content_rect;
	CVI_U32 present_count;
	CVI_U32 write_index;
	CVI_BOOL logged_first_vpss_frame;
	CVI_BOOL owns_sys;
	CVI_BOOL vo_started;
	CVI_BOOL vpss_started;
	CVI_BOOL stage_ready;
	CVI_BOOL vo_ready;
	uint16_t rotate_degrees;
} lintx_vo_ctx_t;

static ROTATION_E rotation_from_degrees(uint16_t degrees)
{
	switch (degrees % 360) {
	case 90:
		return ROTATION_90;
	case 180:
		return ROTATION_180;
	case 270:
		return ROTATION_270;
	default:
		return ROTATION_0;
	}
}

static CVI_U32 aligned_nv21_size(SIZE_S size)
{
	return ALIGN(size.u32Width, DEFAULT_ALIGN) *
	       ALIGN(size.u32Height, DEFAULT_ALIGN) * 3U / 2U;
}

static CVI_BOOL use_vpss_rotation(const lintx_vo_ctx_t *ctx)
{
	switch (ctx->rotate_degrees % 360) {
	case 90:
	case 270:
		return CVI_TRUE;
	default:
		return CVI_FALSE;
	}
}

static CVI_U32 align_even_down(CVI_U32 value)
{
	if (value <= 2)
		return value;
	return value & ~1U;
}

static void compute_centered_rect(const SIZE_S *src, const SIZE_S *dst, RECT_S *rect)
{
	CVI_U32 scaled_width;
	CVI_U32 scaled_height;

	memset(rect, 0, sizeof(*rect));
	if (src->u32Width == 0 || src->u32Height == 0 ||
	    dst->u32Width == 0 || dst->u32Height == 0) {
		rect->u32Width = dst->u32Width;
		rect->u32Height = dst->u32Height;
		return;
	}

	if ((CVI_U64)src->u32Width * dst->u32Height >=
	    (CVI_U64)dst->u32Width * src->u32Height) {
		scaled_width = dst->u32Width;
		scaled_height = (CVI_U32)(((CVI_U64)dst->u32Width * src->u32Height) /
					  src->u32Width);
	} else {
		scaled_height = dst->u32Height;
		scaled_width = (CVI_U32)(((CVI_U64)dst->u32Height * src->u32Width) /
					 src->u32Height);
	}

	scaled_width = align_even_down(scaled_width);
	scaled_height = align_even_down(scaled_height);
	if (scaled_width == 0)
		scaled_width = (dst->u32Width > 1) ? 2 : dst->u32Width;
	if (scaled_height == 0)
		scaled_height = (dst->u32Height > 1) ? 2 : dst->u32Height;
	if (scaled_width > dst->u32Width)
		scaled_width = align_even_down(dst->u32Width);
	if (scaled_height > dst->u32Height)
		scaled_height = align_even_down(dst->u32Height);

	rect->u32Width = scaled_width;
	rect->u32Height = scaled_height;
	rect->s32X = (CVI_S32)((dst->u32Width - rect->u32Width) / 2);
	rect->s32Y = (CVI_S32)((dst->u32Height - rect->u32Height) / 2);
	rect->s32X &= ~1;
	rect->s32Y &= ~1;
}

static CVI_S32 init_system_with_pool(lintx_vo_ctx_t *ctx)
{
	VB_CONFIG_S vb_config;
	VB_CAL_CONFIG_S logical_cfg;
	VB_CAL_CONFIG_S logical_rot_cfg;
	VB_CAL_CONFIG_S panel_cfg;
	VB_CAL_CONFIG_S panel_rot_cfg;
	CVI_U32 max_blk_size;

	memset(&vb_config, 0, sizeof(vb_config));
	memset(&logical_cfg, 0, sizeof(logical_cfg));
	memset(&logical_rot_cfg, 0, sizeof(logical_rot_cfg));
	memset(&panel_cfg, 0, sizeof(panel_cfg));
	memset(&panel_rot_cfg, 0, sizeof(panel_rot_cfg));

	COMMON_GetPicBufferConfig(
		ctx->logical_size.u32Width, ctx->logical_size.u32Height,
		PIXEL_FORMAT_NV21, DATA_BITWIDTH_8, COMPRESS_MODE_NONE, DEFAULT_ALIGN, &logical_cfg);
	COMMON_GetPicBufferConfig(
		ctx->logical_size.u32Height, ctx->logical_size.u32Width,
		PIXEL_FORMAT_NV21, DATA_BITWIDTH_8, COMPRESS_MODE_NONE, DEFAULT_ALIGN, &logical_rot_cfg);
	COMMON_GetPicBufferConfig(
		ctx->panel_size.u32Width, ctx->panel_size.u32Height,
		PIXEL_FORMAT_NV21, DATA_BITWIDTH_8, COMPRESS_MODE_NONE, DEFAULT_ALIGN, &panel_cfg);
	COMMON_GetPicBufferConfig(
		ctx->panel_size.u32Height, ctx->panel_size.u32Width,
		PIXEL_FORMAT_NV21, DATA_BITWIDTH_8, COMPRESS_MODE_NONE, DEFAULT_ALIGN, &panel_rot_cfg);

	max_blk_size = logical_cfg.u32VBSize;
	if (logical_rot_cfg.u32VBSize > max_blk_size)
		max_blk_size = logical_rot_cfg.u32VBSize;
	if (panel_cfg.u32VBSize > max_blk_size)
		max_blk_size = panel_cfg.u32VBSize;
	if (panel_rot_cfg.u32VBSize > max_blk_size)
		max_blk_size = panel_rot_cfg.u32VBSize;
	if (aligned_nv21_size(ctx->logical_size) > max_blk_size)
		max_blk_size = aligned_nv21_size(ctx->logical_size);
	if (aligned_nv21_size(ctx->panel_size) > max_blk_size)
		max_blk_size = aligned_nv21_size(ctx->panel_size);

	vb_config.u32MaxPoolCnt = 1;
	vb_config.astCommPool[0].u32BlkSize = max_blk_size;
	vb_config.astCommPool[0].u32BlkCnt = 12;
	vb_config.astCommPool[0].enRemapMode = VB_REMAP_MODE_CACHED;

	SAMPLE_PRT("vpss/vo init logical=%ux%u panel=%ux%u common_nv21_blk=%u\n",
		   ctx->logical_size.u32Width, ctx->logical_size.u32Height,
		   ctx->panel_size.u32Width, ctx->panel_size.u32Height,
		   max_blk_size);
	return SAMPLE_COMM_SYS_Init(&vb_config);
}

static CVI_S32 create_stage_pool(lintx_vo_ctx_t *ctx)
{
	VB_POOL_CONFIG_S pool_cfg;
	VB_CAL_CONFIG_S cal_cfg;

	memset(&pool_cfg, 0, sizeof(pool_cfg));
	memset(&cal_cfg, 0, sizeof(cal_cfg));

	COMMON_GetPicBufferConfig(
		ctx->logical_size.u32Width, ctx->logical_size.u32Height,
		PIXEL_FORMAT_RGB_888, DATA_BITWIDTH_8, COMPRESS_MODE_NONE, DEFAULT_ALIGN, &cal_cfg);

	ctx->stage_blk_size = cal_cfg.u32VBSize;
	ctx->stage_stride = cal_cfg.u32MainStride;
	ctx->stage_map_len = cal_cfg.u32MainYSize;

	pool_cfg.u32BlkSize = ctx->stage_blk_size;
	pool_cfg.u32BlkCnt = LINTX_STAGE_FRAME_COUNT;
	pool_cfg.enRemapMode = VB_REMAP_MODE_CACHED;

	ctx->stage_pool = CVI_VB_CreatePool(&pool_cfg);
	if (ctx->stage_pool == VB_INVALID_POOLID)
		return CVI_FAILURE;

	SAMPLE_PRT("stage rgb pool id=%d blk_size=%u stride=%u map_len=%u\n",
		   ctx->stage_pool, ctx->stage_blk_size, ctx->stage_stride, ctx->stage_map_len);
	return CVI_SUCCESS;
}

static CVI_S32 create_vo_pool(lintx_vo_ctx_t *ctx)
{
	VB_POOL_CONFIG_S pool_cfg;
	VB_CAL_CONFIG_S cal_cfg;

	memset(&pool_cfg, 0, sizeof(pool_cfg));
	memset(&cal_cfg, 0, sizeof(cal_cfg));

	COMMON_GetPicBufferConfig(
		ctx->output_size.u32Width, ctx->output_size.u32Height,
		PIXEL_FORMAT_NV21, DATA_BITWIDTH_8, COMPRESS_MODE_NONE, DEFAULT_ALIGN, &cal_cfg);

	ctx->vo_blk_size = cal_cfg.u32VBSize;
	ctx->vo_stride_y = cal_cfg.u32MainStride;
	ctx->vo_stride_uv = cal_cfg.u32CStride;
	ctx->vo_len_y = cal_cfg.u32MainYSize;
	ctx->vo_len_uv = cal_cfg.u32MainCSize;

	pool_cfg.u32BlkSize = ctx->vo_blk_size;
	pool_cfg.u32BlkCnt = 1;
	pool_cfg.enRemapMode = VB_REMAP_MODE_CACHED;

	ctx->vo_pool = CVI_VB_CreatePool(&pool_cfg);
	if (ctx->vo_pool == VB_INVALID_POOLID)
		return CVI_FAILURE;

	SAMPLE_PRT("vo nv21 pool id=%d blk_size=%u stride=%u/%u len=%u/%u\n",
		   ctx->vo_pool, ctx->vo_blk_size, ctx->vo_stride_y, ctx->vo_stride_uv,
		   ctx->vo_len_y, ctx->vo_len_uv);
	return CVI_SUCCESS;
}

static CVI_S32 ensure_stage_pool_ready(lintx_vo_ctx_t *ctx)
{
	if (create_stage_pool(ctx) == CVI_SUCCESS)
		return CVI_SUCCESS;

	if (init_system_with_pool(ctx) != CVI_SUCCESS)
		return CVI_FAILURE;

	ctx->owns_sys = CVI_TRUE;
	return create_stage_pool(ctx);
}

static CVI_S32 ensure_vo_pool_ready(lintx_vo_ctx_t *ctx)
{
	if (create_vo_pool(ctx) == CVI_SUCCESS)
		return CVI_SUCCESS;

	if (!ctx->owns_sys && init_system_with_pool(ctx) != CVI_SUCCESS)
		return CVI_FAILURE;

	ctx->owns_sys = CVI_TRUE;
	return create_vo_pool(ctx);
}

static CVI_S32 prepare_stage_frame(lintx_vo_ctx_t *ctx)
{
	VB_CAL_CONFIG_S cal_cfg;
	CVI_U32 idx;

	memset(&cal_cfg, 0, sizeof(cal_cfg));
	COMMON_GetPicBufferConfig(
		ctx->logical_size.u32Width, ctx->logical_size.u32Height,
		PIXEL_FORMAT_RGB_888, DATA_BITWIDTH_8, COMPRESS_MODE_NONE, DEFAULT_ALIGN, &cal_cfg);

	for (idx = 0; idx < LINTX_STAGE_FRAME_COUNT; ++idx) {
		VB_BLK blk;
		CVI_U64 phy_addr;

		memset(&ctx->stage_frames[idx].frame, 0, sizeof(ctx->stage_frames[idx].frame));
		blk = CVI_VB_GetBlock(ctx->stage_pool, ctx->stage_blk_size);
		if (blk == VB_INVALID_HANDLE)
			return CVI_FAILURE;

		phy_addr = CVI_VB_Handle2PhysAddr(blk);
		ctx->stage_frames[idx].virt = CVI_SYS_MmapCache(phy_addr, ctx->stage_map_len);
		if (ctx->stage_frames[idx].virt == CVI_NULL) {
			CVI_VB_ReleaseBlock(blk);
			return CVI_FAILURE;
		}

		ctx->stage_frames[idx].frame.u32PoolId = CVI_VB_Handle2PoolId(blk);
		ctx->stage_frames[idx].frame.stVFrame.enCompressMode = COMPRESS_MODE_NONE;
		ctx->stage_frames[idx].frame.stVFrame.enPixelFormat = PIXEL_FORMAT_RGB_888;
		ctx->stage_frames[idx].frame.stVFrame.enVideoFormat = VIDEO_FORMAT_LINEAR;
		ctx->stage_frames[idx].frame.stVFrame.enColorGamut = COLOR_GAMUT_BT601;
		ctx->stage_frames[idx].frame.stVFrame.enDynamicRange = DYNAMIC_RANGE_SDR8;
		ctx->stage_frames[idx].frame.stVFrame.u32Width = ctx->logical_size.u32Width;
		ctx->stage_frames[idx].frame.stVFrame.u32Height = ctx->logical_size.u32Height;
		ctx->stage_frames[idx].frame.stVFrame.u32Stride[0] = cal_cfg.u32MainStride;
		ctx->stage_frames[idx].frame.stVFrame.u32Length[0] = cal_cfg.u32MainYSize;
		ctx->stage_frames[idx].frame.stVFrame.u64PhyAddr[0] = phy_addr;
		ctx->stage_frames[idx].frame.stVFrame.pu8VirAddr[0] = ctx->stage_frames[idx].virt;

		memset(ctx->stage_frames[idx].virt, 0, ctx->stage_map_len);
	}

	ctx->write_index = 0;
	ctx->stage_ready = CVI_TRUE;
	return CVI_SUCCESS;
}

static void release_stage_frame(lintx_vo_ctx_t *ctx)
{
	CVI_U32 idx;

	for (idx = 0; idx < LINTX_STAGE_FRAME_COUNT; ++idx) {
		VB_BLK blk;

		if (ctx->stage_frames[idx].virt != CVI_NULL && ctx->stage_map_len != 0)
			CVI_SYS_Munmap(ctx->stage_frames[idx].virt, ctx->stage_map_len);

		ctx->stage_frames[idx].virt = CVI_NULL;

		blk = CVI_VB_PhysAddr2Handle(ctx->stage_frames[idx].frame.stVFrame.u64PhyAddr[0]);
		if (blk != VB_INVALID_HANDLE)
			CVI_VB_ReleaseBlock(blk);

		memset(&ctx->stage_frames[idx].frame, 0, sizeof(ctx->stage_frames[idx].frame));
	}
	ctx->stage_ready = CVI_FALSE;
}

static CVI_S32 prepare_vo_frame(lintx_vo_ctx_t *ctx)
{
	VB_BLK blk;
	CVI_U64 phy_addr;
	CVI_U32 chroma_offset;
	CVI_VOID *virt_base;

	memset(&ctx->vo_frame, 0, sizeof(ctx->vo_frame));
	blk = CVI_VB_GetBlock(ctx->vo_pool, ctx->vo_blk_size);
	if (blk == VB_INVALID_HANDLE)
		return CVI_FAILURE;

	phy_addr = CVI_VB_Handle2PhysAddr(blk);
	ctx->vo_map_len = ctx->vo_blk_size;
	virt_base = CVI_SYS_MmapCache(phy_addr, ctx->vo_map_len);
	if (virt_base == CVI_NULL) {
		CVI_VB_ReleaseBlock(blk);
		return CVI_FAILURE;
	}

	chroma_offset = ALIGN(ctx->vo_len_y, DEFAULT_ALIGN);
	ctx->vo_virt_base = virt_base;
	ctx->vo_virt_y = (CVI_U8 *)virt_base;
	ctx->vo_virt_uv = (CVI_U8 *)virt_base + chroma_offset;

	ctx->vo_frame.u32PoolId = CVI_VB_Handle2PoolId(blk);
	ctx->vo_frame.stVFrame.enCompressMode = COMPRESS_MODE_NONE;
	ctx->vo_frame.stVFrame.enPixelFormat = PIXEL_FORMAT_NV21;
	ctx->vo_frame.stVFrame.enVideoFormat = VIDEO_FORMAT_LINEAR;
	ctx->vo_frame.stVFrame.enColorGamut = COLOR_GAMUT_BT601;
	ctx->vo_frame.stVFrame.enDynamicRange = DYNAMIC_RANGE_SDR8;
	ctx->vo_frame.stVFrame.u32Width = ctx->output_size.u32Width;
	ctx->vo_frame.stVFrame.u32Height = ctx->output_size.u32Height;
	ctx->vo_frame.stVFrame.u32Stride[0] = ctx->vo_stride_y;
	ctx->vo_frame.stVFrame.u32Stride[1] = ctx->vo_stride_uv;
	ctx->vo_frame.stVFrame.u32Length[0] = ctx->vo_len_y;
	ctx->vo_frame.stVFrame.u32Length[1] = ctx->vo_len_uv;
	ctx->vo_frame.stVFrame.u64PhyAddr[0] = phy_addr;
	ctx->vo_frame.stVFrame.u64PhyAddr[1] = phy_addr + chroma_offset;
	ctx->vo_frame.stVFrame.pu8VirAddr[0] = ctx->vo_virt_y;
	ctx->vo_frame.stVFrame.pu8VirAddr[1] = ctx->vo_virt_uv;
	ctx->vo_ready = CVI_TRUE;

	memset(ctx->vo_virt_y, 0x00, ctx->vo_len_y);
	memset(ctx->vo_virt_uv, 0x80, ctx->vo_len_uv);
	CVI_SYS_IonFlushCache(ctx->vo_frame.stVFrame.u64PhyAddr[0], ctx->vo_virt_y, ctx->vo_len_y);
	CVI_SYS_IonFlushCache(ctx->vo_frame.stVFrame.u64PhyAddr[1], ctx->vo_virt_uv, ctx->vo_len_uv);

	SAMPLE_PRT("vo stage ready %ux%u stride=%u/%u len=%u/%u\n",
		   ctx->vo_frame.stVFrame.u32Width, ctx->vo_frame.stVFrame.u32Height,
		   ctx->vo_frame.stVFrame.u32Stride[0], ctx->vo_frame.stVFrame.u32Stride[1],
		   ctx->vo_frame.stVFrame.u32Length[0], ctx->vo_frame.stVFrame.u32Length[1]);
	return CVI_SUCCESS;
}

static void release_vo_frame(lintx_vo_ctx_t *ctx)
{
	VB_BLK blk;

	if (ctx->vo_virt_base != CVI_NULL)
		CVI_SYS_Munmap(ctx->vo_virt_base, ctx->vo_map_len);

	ctx->vo_virt_base = CVI_NULL;
	ctx->vo_virt_y = CVI_NULL;
	ctx->vo_virt_uv = CVI_NULL;
	blk = CVI_VB_PhysAddr2Handle(ctx->vo_frame.stVFrame.u64PhyAddr[0]);
	if (blk != VB_INVALID_HANDLE)
		CVI_VB_ReleaseBlock(blk);

	memset(&ctx->vo_frame, 0, sizeof(ctx->vo_frame));
	ctx->vo_ready = CVI_FALSE;
}

static CVI_S32 init_vpss(lintx_vo_ctx_t *ctx)
{
	VPSS_GRP_ATTR_S grp_attr;
	VPSS_CHN_ATTR_S chn_attr[VPSS_MAX_PHY_CHN_NUM];
	CVI_BOOL chn_enable[VPSS_MAX_PHY_CHN_NUM] = {0};
	VPSS_GRP vpss_grp;

	memset(&grp_attr, 0, sizeof(grp_attr));
	memset(chn_attr, 0, sizeof(chn_attr));

	vpss_grp = CVI_VPSS_GetAvailableGrp();
	if (vpss_grp < 0) {
		SAMPLE_PRT("no available VPSS group, fallback to grp0\n");
		vpss_grp = 0;
		CVI_VPSS_DestroyGrp(vpss_grp);
	}
	ctx->vpss_grp = vpss_grp;

	grp_attr.stFrameRate.s32SrcFrameRate = -1;
	grp_attr.stFrameRate.s32DstFrameRate = -1;
	grp_attr.enPixelFormat = PIXEL_FORMAT_RGB_888;
	grp_attr.u32MaxW = ctx->logical_size.u32Width;
	grp_attr.u32MaxH = ctx->logical_size.u32Height;
	grp_attr.u8VpssDev = 0;

	chn_enable[LINTX_VPSS_CHN] = CVI_TRUE;
	chn_attr[LINTX_VPSS_CHN].u32Width = ctx->output_size.u32Width;
	chn_attr[LINTX_VPSS_CHN].u32Height = ctx->output_size.u32Height;
	chn_attr[LINTX_VPSS_CHN].enVideoFormat = VIDEO_FORMAT_LINEAR;
	chn_attr[LINTX_VPSS_CHN].enPixelFormat = PIXEL_FORMAT_NV21;
	chn_attr[LINTX_VPSS_CHN].stFrameRate.s32SrcFrameRate = -1;
	chn_attr[LINTX_VPSS_CHN].stFrameRate.s32DstFrameRate = -1;
	chn_attr[LINTX_VPSS_CHN].u32Depth = 3;
	chn_attr[LINTX_VPSS_CHN].bMirror = CVI_FALSE;
	chn_attr[LINTX_VPSS_CHN].bFlip = CVI_FALSE;
	chn_attr[LINTX_VPSS_CHN].stAspectRatio.enMode = ASPECT_RATIO_NONE;
	chn_attr[LINTX_VPSS_CHN].stAspectRatio.bEnableBgColor = CVI_TRUE;
	chn_attr[LINTX_VPSS_CHN].stAspectRatio.u32BgColor = COLOR_RGB_BLACK;
	chn_attr[LINTX_VPSS_CHN].stAspectRatio.stVideoRect.s32X = 0;
	chn_attr[LINTX_VPSS_CHN].stAspectRatio.stVideoRect.s32Y = 0;
	chn_attr[LINTX_VPSS_CHN].stAspectRatio.stVideoRect.u32Width =
		chn_attr[LINTX_VPSS_CHN].u32Width;
	chn_attr[LINTX_VPSS_CHN].stAspectRatio.stVideoRect.u32Height =
		chn_attr[LINTX_VPSS_CHN].u32Height;
	chn_attr[LINTX_VPSS_CHN].stNormalize.bEnable = CVI_FALSE;

	if (SAMPLE_COMM_VPSS_Init(ctx->vpss_grp, chn_enable, &grp_attr, chn_attr) != CVI_SUCCESS)
		return CVI_FAILURE;
	if (SAMPLE_COMM_VPSS_Start(ctx->vpss_grp, chn_enable, &grp_attr, chn_attr) != CVI_SUCCESS)
		return CVI_FAILURE;
	if (use_vpss_rotation(ctx) &&
	    CVI_VPSS_SetChnRotation(ctx->vpss_grp, LINTX_VPSS_CHN,
				    rotation_from_degrees(ctx->rotate_degrees)) != CVI_SUCCESS)
		return CVI_FAILURE;

	SAMPLE_PRT("VPSS ready grp=%d input=RGB888 output=NV21 size=%ux%u rotate=%u via=%s\n",
		   ctx->vpss_grp, ctx->output_size.u32Width, ctx->output_size.u32Height,
		   ctx->rotate_degrees, use_vpss_rotation(ctx) ? "vpss" : "none");
	ctx->vpss_started = CVI_TRUE;
	return CVI_SUCCESS;
}

static void build_vo_padded_frame(lintx_vo_ctx_t *ctx, const VIDEO_FRAME_INFO_S *src_frame)
{
	const VIDEO_FRAME_S *src = &src_frame->stVFrame;
	VIDEO_FRAME_S *dst = &ctx->vo_frame.stVFrame;
	const CVI_U8 *src_y;
	const CVI_U8 *src_uv;
	CVI_BOOL mapped_y = CVI_FALSE;
	CVI_BOOL mapped_uv = CVI_FALSE;
	CVI_U32 copy_width;
	CVI_U32 copy_height;
	CVI_U32 row;

	if (src->pu8VirAddr[0] == CVI_NULL) {
		CVI_SYS_IonInvalidateCache(src->u64PhyAddr[0], NULL, src->u32Length[0]);
		src_y = CVI_SYS_Mmap(src->u64PhyAddr[0], src->u32Length[0]);
		mapped_y = CVI_TRUE;
	} else {
		CVI_SYS_IonInvalidateCache(src->u64PhyAddr[0], src->pu8VirAddr[0], src->u32Length[0]);
		src_y = src->pu8VirAddr[0];
	}

	if (src->pu8VirAddr[1] == CVI_NULL) {
		CVI_SYS_IonInvalidateCache(src->u64PhyAddr[1], NULL, src->u32Length[1]);
		src_uv = CVI_SYS_Mmap(src->u64PhyAddr[1], src->u32Length[1]);
		mapped_uv = CVI_TRUE;
	} else {
		CVI_SYS_IonInvalidateCache(src->u64PhyAddr[1], src->pu8VirAddr[1], src->u32Length[1]);
		src_uv = src->pu8VirAddr[1];
	}

	if (src_y == CVI_NULL || src_uv == CVI_NULL) {
		if (mapped_y && src_y != CVI_NULL)
			CVI_SYS_Munmap((void *)src_y, src->u32Length[0]);
		if (mapped_uv && src_uv != CVI_NULL)
			CVI_SYS_Munmap((void *)src_uv, src->u32Length[1]);
		return;
	}

	memset(ctx->vo_virt_y, 0x00, ctx->vo_len_y);
	memset(ctx->vo_virt_uv, 0x80, ctx->vo_len_uv);

	copy_width = src->u32Width;
	if (copy_width > ctx->content_rect.u32Width)
		copy_width = ctx->content_rect.u32Width;
	copy_height = src->u32Height;
	if (copy_height > ctx->content_rect.u32Height)
		copy_height = ctx->content_rect.u32Height;

	for (row = 0; row < copy_height; ++row) {
		CVI_U8 *dst_row = ctx->vo_virt_y +
				 (ctx->content_rect.s32Y + row) * dst->u32Stride[0] +
				 ctx->content_rect.s32X;
		const CVI_U8 *src_row = src_y + row * src->u32Stride[0];
		memcpy(dst_row, src_row, copy_width);
	}

	for (row = 0; row < (copy_height / 2); ++row) {
		CVI_U8 *dst_row = ctx->vo_virt_uv +
				 ((ctx->content_rect.s32Y / 2) + row) * dst->u32Stride[1] +
				 ctx->content_rect.s32X;
		const CVI_U8 *src_row = src_uv + row * src->u32Stride[1];
		memcpy(dst_row, src_row, copy_width);
	}

	dst->u64PTS = src->u64PTS;
	dst->u32TimeRef = src->u32TimeRef;
	CVI_SYS_IonFlushCache(dst->u64PhyAddr[0], ctx->vo_virt_y, ctx->vo_len_y);
	CVI_SYS_IonFlushCache(dst->u64PhyAddr[1], ctx->vo_virt_uv, ctx->vo_len_uv);

	if (mapped_y)
		CVI_SYS_Munmap((void *)src_y, src->u32Length[0]);
	if (mapped_uv)
		CVI_SYS_Munmap((void *)src_uv, src->u32Length[1]);
}

static CVI_S32 init_vo(lintx_vo_ctx_t *ctx)
{
	VO_CHN_ATTR_S chn_attr;
	CVI_S32 ret;

	memset(&ctx->vo_config, 0, sizeof(ctx->vo_config));
	ret = SAMPLE_COMM_VO_GetDefConfig(&ctx->vo_config);
	if (ret != CVI_SUCCESS)
		return ret;

	ctx->vo_config.VoDev = 0;
	ctx->vo_config.stVoPubAttr.enIntfType = VO_INTF_MIPI;
	ctx->vo_config.stVoPubAttr.enIntfSync = VO_OUTPUT_480x800_60;
	ctx->vo_config.stDispRect.s32X = 0;
	ctx->vo_config.stDispRect.s32Y = 0;
	ctx->vo_config.stDispRect.u32Width = ctx->panel_size.u32Width;
	ctx->vo_config.stDispRect.u32Height = ctx->panel_size.u32Height;
	ctx->vo_config.stImageSize = ctx->panel_size;
	ctx->vo_config.enPixFormat = PIXEL_FORMAT_NV21;
	ctx->vo_config.enVoMode = VO_MODE_1MUX;
	ctx->vo_config.u32DisBufLen = 3;

	ret = SAMPLE_COMM_VO_StartVO(&ctx->vo_config);
	if (ret != CVI_SUCCESS)
		return ret;
	ctx->vo_started = CVI_TRUE;

	memset(&chn_attr, 0, sizeof(chn_attr));
	chn_attr.u32Priority = 0;
	chn_attr.stRect.s32X = 0;
	chn_attr.stRect.s32Y = 0;
	chn_attr.stRect.u32Width = ctx->panel_size.u32Width;
	chn_attr.stRect.u32Height = ctx->panel_size.u32Height;

	CVI_VO_DisableChn(ctx->vo_config.VoDev, LINTX_VO_CHN);
	ret = CVI_VO_SetChnAttr(ctx->vo_config.VoDev, LINTX_VO_CHN, &chn_attr);
	if (ret != CVI_SUCCESS)
		return ret;
	ret = CVI_VO_EnableChn(ctx->vo_config.VoDev, LINTX_VO_CHN);
	if (ret != CVI_SUCCESS)
		return ret;
	SAMPLE_PRT("VO ready dev=%d rect=%ux%u rotate=%u via=%s\n",
		   ctx->vo_config.VoDev,
		   chn_attr.stRect.u32Width, chn_attr.stRect.u32Height,
		   ctx->rotate_degrees,
		   use_vpss_rotation(ctx) ? "vpss" : "vo-disabled");
	return CVI_SUCCESS;
}

void *lintx_vo_create(uint32_t logical_width, uint32_t logical_height,
		      uint32_t panel_width, uint32_t panel_height,
		      uint16_t rotate_degrees)
{
	lintx_vo_ctx_t *ctx;

	ctx = calloc(1, sizeof(*ctx));
	if (ctx == NULL)
		return NULL;

	ctx->logical_size.u32Width = logical_width;
	ctx->logical_size.u32Height = logical_height;
	ctx->panel_size.u32Width = panel_width;
	ctx->panel_size.u32Height = panel_height;
	ctx->rotate_degrees = rotate_degrees;
	if (use_vpss_rotation(ctx)) {
		ctx->output_size = ctx->panel_size;
	} else {
		ctx->output_size = ctx->logical_size;
	}
	ctx->stage_pool = VB_INVALID_POOLID;
	ctx->vo_pool = VB_INVALID_POOLID;
	{
		SIZE_S src_size = ctx->logical_size;
		if (use_vpss_rotation(ctx)) {
			src_size.u32Width = logical_height;
			src_size.u32Height = logical_width;
		}
		compute_centered_rect(&src_size, &ctx->output_size, &ctx->content_rect);
	}

	if (ensure_stage_pool_ready(ctx) != CVI_SUCCESS)
		goto fail;
	if (prepare_stage_frame(ctx) != CVI_SUCCESS)
		goto fail;
	if (ensure_vo_pool_ready(ctx) != CVI_SUCCESS)
		goto fail;
	if (prepare_vo_frame(ctx) != CVI_SUCCESS)
		goto fail;
	if (init_vpss(ctx) != CVI_SUCCESS)
		goto fail;
	if (init_vo(ctx) != CVI_SUCCESS)
		goto fail;

	return ctx;

fail:
	if (ctx->vo_started)
		SAMPLE_COMM_VO_StopVO(&ctx->vo_config);
	if (ctx->vpss_started) {
		CVI_BOOL chn_enable[VPSS_MAX_PHY_CHN_NUM] = {0};
		chn_enable[LINTX_VPSS_CHN] = CVI_TRUE;
		SAMPLE_COMM_VPSS_Stop(ctx->vpss_grp, chn_enable);
	}
	if (ctx->stage_ready)
		release_stage_frame(ctx);
	if (ctx->vo_ready)
		release_vo_frame(ctx);
	if (ctx->stage_pool != VB_INVALID_POOLID)
		CVI_VB_DestroyPool(ctx->stage_pool);
	if (ctx->vo_pool != VB_INVALID_POOLID)
		CVI_VB_DestroyPool(ctx->vo_pool);
	if (ctx->owns_sys)
		SAMPLE_COMM_SYS_Exit();
	free(ctx);
	return NULL;
}

uint8_t *lintx_vo_framebuffer_ptr(void *handle)
{
	lintx_vo_ctx_t *ctx = (lintx_vo_ctx_t *)handle;

	if (ctx == NULL || !ctx->stage_ready)
		return NULL;
	return ctx->stage_frames[ctx->write_index].virt;
}

size_t lintx_vo_framebuffer_len(void *handle)
{
	lintx_vo_ctx_t *ctx = (lintx_vo_ctx_t *)handle;

	if (ctx == NULL || !ctx->stage_ready)
		return 0;
	return ctx->stage_map_len;
}

uint32_t lintx_vo_framebuffer_stride(void *handle)
{
	lintx_vo_ctx_t *ctx = (lintx_vo_ctx_t *)handle;

	if (ctx == NULL || !ctx->stage_ready)
		return 0;
	return ctx->stage_stride;
}

int lintx_vo_present(void *handle)
{
	lintx_vo_ctx_t *ctx = (lintx_vo_ctx_t *)handle;
	lintx_stage_frame_t *stage;
	VIDEO_FRAME_INFO_S vpss_frame;
	CVI_S32 ret;

	if (ctx == NULL || !ctx->stage_ready) {
		errno = EINVAL;
		return -1;
	}

	stage = &ctx->stage_frames[ctx->write_index];
	CVI_SYS_IonFlushCache(stage->frame.stVFrame.u64PhyAddr[0],
			      stage->virt, ctx->stage_map_len);

	ret = CVI_VPSS_SendFrame(ctx->vpss_grp, &stage->frame, 1000);
	if (ret != CVI_SUCCESS) {
		SAMPLE_PRT("CVI_VPSS_SendFrame failed grp=%d ret=%#x w=%u h=%u stride=%u len=%u fmt=%d\n",
			   ctx->vpss_grp, ret,
			   stage->frame.stVFrame.u32Width,
			   stage->frame.stVFrame.u32Height,
			   stage->frame.stVFrame.u32Stride[0],
			   stage->frame.stVFrame.u32Length[0],
			   stage->frame.stVFrame.enPixelFormat);
		usleep(5000);
		errno = EIO;
		return -1;
	}

	memset(&vpss_frame, 0, sizeof(vpss_frame));
	ret = CVI_VPSS_GetChnFrame(ctx->vpss_grp, LINTX_VPSS_CHN, &vpss_frame, 1000);
	if (ret != CVI_SUCCESS) {
		SAMPLE_PRT("CVI_VPSS_GetChnFrame failed grp=%d chn=%d ret=%#x\n",
			   ctx->vpss_grp, LINTX_VPSS_CHN, ret);
		usleep(5000);
		errno = EAGAIN;
		return -1;
	}

	if (!ctx->logged_first_vpss_frame) {
		SAMPLE_PRT("first vpss frame w=%u h=%u fmt=%d stride=%u/%u len=%u/%u\n",
			   vpss_frame.stVFrame.u32Width,
			   vpss_frame.stVFrame.u32Height,
			   vpss_frame.stVFrame.enPixelFormat,
			   vpss_frame.stVFrame.u32Stride[0],
			   vpss_frame.stVFrame.u32Stride[1],
			   vpss_frame.stVFrame.u32Length[0],
			   vpss_frame.stVFrame.u32Length[1]);
		ctx->logged_first_vpss_frame = CVI_TRUE;
	}

	if (!ctx->vo_ready) {
		CVI_VPSS_ReleaseChnFrame(ctx->vpss_grp, LINTX_VPSS_CHN, &vpss_frame);
		errno = EINVAL;
		return -1;
	}

	build_vo_padded_frame(ctx, &vpss_frame);
	ret = CVI_VO_SendFrame(ctx->vo_config.VoDev, LINTX_VO_CHN, &ctx->vo_frame, 1000);
	if (ret != CVI_SUCCESS) {
		SAMPLE_PRT("CVI_VO_SendFrame failed dev=%d chn=%d ret=%#x w=%u h=%u fmt=%d stride=%u/%u\n",
			   ctx->vo_config.VoDev, LINTX_VO_CHN, ret,
			   ctx->vo_frame.stVFrame.u32Width,
			   ctx->vo_frame.stVFrame.u32Height,
			   ctx->vo_frame.stVFrame.enPixelFormat,
			   ctx->vo_frame.stVFrame.u32Stride[0],
			   ctx->vo_frame.stVFrame.u32Stride[1]);
		CVI_VPSS_ReleaseChnFrame(ctx->vpss_grp, LINTX_VPSS_CHN, &vpss_frame);
		usleep(5000);
		errno = EIO;
		return -1;
	}

	ctx->present_count++;
	if (ctx->present_count <= 3 || ctx->present_count % 120 == 0) {
		SAMPLE_PRT("display frame ok grp=%d count=%u stage=%ux%u/%d vpss=%ux%u/%d vo=%ux%u/%d dev=%d\n",
			   ctx->vpss_grp, ctx->present_count,
			   stage->frame.stVFrame.u32Width,
			   stage->frame.stVFrame.u32Height,
			   stage->frame.stVFrame.enPixelFormat,
			   vpss_frame.stVFrame.u32Width,
			   vpss_frame.stVFrame.u32Height,
			   vpss_frame.stVFrame.enPixelFormat,
			   ctx->vo_frame.stVFrame.u32Width,
			   ctx->vo_frame.stVFrame.u32Height,
			   ctx->vo_frame.stVFrame.enPixelFormat,
			   ctx->vo_config.VoDev);
	}

	CVI_VPSS_ReleaseChnFrame(ctx->vpss_grp, LINTX_VPSS_CHN, &vpss_frame);
	ctx->write_index = (ctx->write_index + 1) % LINTX_STAGE_FRAME_COUNT;

	return 0;
}

void lintx_vo_destroy(void *handle)
{
	lintx_vo_ctx_t *ctx = (lintx_vo_ctx_t *)handle;

	if (ctx == NULL)
		return;

	if (ctx->vo_started)
		SAMPLE_COMM_VO_StopVO(&ctx->vo_config);
	if (ctx->vpss_started) {
		CVI_BOOL chn_enable[VPSS_MAX_PHY_CHN_NUM] = {0};
		chn_enable[LINTX_VPSS_CHN] = CVI_TRUE;
		SAMPLE_COMM_VPSS_Stop(ctx->vpss_grp, chn_enable);
	}
	if (ctx->stage_ready)
		release_stage_frame(ctx);
	if (ctx->vo_ready)
		release_vo_frame(ctx);
	if (ctx->stage_pool != VB_INVALID_POOLID)
		CVI_VB_DestroyPool(ctx->stage_pool);
	if (ctx->vo_pool != VB_INVALID_POOLID)
		CVI_VB_DestroyPool(ctx->vo_pool);
	if (ctx->owns_sys)
		SAMPLE_COMM_SYS_Exit();
	free(ctx);
}
