#define _GNU_SOURCE
#include <pipewire/pipewire.h>
#include <spa/param/audio/format-utils.h>
#include <spa/utils/result.h>
#include <stdlib.h>
#include <string.h>
#include <errno.h>
typedef void (*fill_fn)(void *, float *, size_t);
struct source { struct pw_thread_loop *loop; struct pw_stream *stream; fill_fn fill; void *userdata; };
static void process(void *userdata) {
 struct source *s=userdata; struct pw_buffer *b=pw_stream_dequeue_buffer(s->stream); if (!b) return;
 if (b->buffer->n_datas==0) { pw_stream_queue_buffer(s->stream,b); return; }
 struct spa_data *d=&b->buffer->datas[0];
 if (d->data && d->chunk) {
  size_t frames=d->maxsize/sizeof(float); if(b->requested && frames>b->requested)frames=b->requested;
  s->fill(s->userdata,d->data,frames); d->chunk->offset=0;d->chunk->stride=sizeof(float);d->chunk->size=frames*sizeof(float);b->size=frames;
 }
 pw_stream_queue_buffer(s->stream,b);
}
static const struct pw_stream_events events={PW_VERSION_STREAM_EVENTS,.process=process};
void *linmic_pw_create(const char *name,const char *description,fill_fn fill,void *userdata) {
 pw_init(NULL,NULL);struct source *s=calloc(1,sizeof(*s));if(!s)return NULL;s->fill=fill;s->userdata=userdata;
 s->loop=pw_thread_loop_new("linmic-pipewire",NULL);if(!s->loop)goto fail;
 s->stream=pw_stream_new_simple(pw_thread_loop_get_loop(s->loop),"LinMic",pw_properties_new(
 PW_KEY_MEDIA_TYPE,"Audio",PW_KEY_MEDIA_CATEGORY,"Capture",PW_KEY_MEDIA_ROLE,"Communication",PW_KEY_MEDIA_CLASS,"Audio/Source",
 "priority.session","1",PW_KEY_NODE_NAME,name,PW_KEY_NODE_DESCRIPTION,description,PW_KEY_NODE_VIRTUAL,"true","node.always-process","true","node.pause-on-idle","false",
 PW_KEY_NODE_LATENCY,"480/48000","audio.rate","48000","audio.channels","1",NULL),&events,s);
 if(!s->stream)goto fail;
 uint8_t buffer[1024];struct spa_pod_builder builder=SPA_POD_BUILDER_INIT(buffer,sizeof(buffer));
 struct spa_audio_info_raw format={.format=SPA_AUDIO_FORMAT_F32_LE,.rate=48000,.channels=1,.position={SPA_AUDIO_CHANNEL_MONO}};
 const struct spa_pod *params[1]={spa_format_audio_raw_build(&builder,SPA_PARAM_EnumFormat,&format)};
 if(pw_stream_connect(s->stream,PW_DIRECTION_OUTPUT,PW_ID_ANY,PW_STREAM_FLAG_MAP_BUFFERS|PW_STREAM_FLAG_RT_PROCESS,params,1)<0)goto fail;
 if(pw_thread_loop_start(s->loop)<0)goto fail;
 return s;
fail: if(s->stream)pw_stream_destroy(s->stream);if(s->loop)pw_thread_loop_destroy(s->loop);free(s);return NULL;
}
int linmic_pw_state(void *p){struct source*s=p;pw_thread_loop_lock(s->loop);int state=pw_stream_get_state(s->stream,NULL);pw_thread_loop_unlock(s->loop);return state;}
uint32_t linmic_pw_id(void *p){struct source*s=p;pw_thread_loop_lock(s->loop);uint32_t id=pw_stream_get_node_id(s->stream);pw_thread_loop_unlock(s->loop);return id;}
void linmic_pw_description(void *p,const char *description){struct source*s=p;pw_thread_loop_lock(s->loop);struct spa_dict_item item=SPA_DICT_ITEM_INIT(PW_KEY_NODE_DESCRIPTION,description);struct spa_dict dict=SPA_DICT_INIT(&item,1);pw_stream_update_properties(s->stream,&dict);pw_thread_loop_unlock(s->loop);}
void linmic_pw_destroy(void *p){struct source*s=p;pw_thread_loop_stop(s->loop);pw_stream_destroy(s->stream);pw_thread_loop_destroy(s->loop);free(s);}
