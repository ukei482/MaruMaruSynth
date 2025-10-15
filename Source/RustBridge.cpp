//RustBridge.cpp

#include "RustBridge.h"
#include <iostream>
#include <cstring>

/*
  RustBridge �����iWindows �p�j
  - LoadLibrary / GetProcAddress �ŃV���{������������
  - ctx�ivoid*�j�� Rust ���Ԃ��Ă���s�����|�C���^�B�Ăяo����͏�� ctx ���ŏ��̈����Ɏ��B
*/

RustBridge::RustBridge() 
    : dll(nullptr), ctx(nullptr),
    fn_create_ctx(nullptr), 
    fn_destroy_ctx(nullptr),
    fn_analyze_file(nullptr),
    fn_set_params(nullptr),
    fn_note_on(nullptr), 
    fn_note_off(nullptr),
    fn_process(nullptr) ,
    fn_log_message(nullptr)
{
}

RustBridge::~RustBridge() { unload(); }

/*
  DLL �����[�h���A�K�v�ȃV���{�����擾����B
  �߂�l�͑S�V���{�������������ǂ����B
*/
bool RustBridge::load(const char* dllPath) //PluginProcessor.cpp�ŌĂ΂�Ă���
{
    dll = LoadLibraryA(dllPath);
    if (!dll) {
        std::cerr << "RustBridge: DLL load failed: DLL�̓ǂݍ��݂Ɏ��s���܂��� " << dllPath << std::endl;
        return false;
    }

    // �V���{������ Rust ���� pub extern "C" �Ƃ��Č��J�������O�ƈ�v������
    fn_create_ctx    = (FnCreateCtx)GetProcAddress   (dll, "mm_create_context");
    fn_destroy_ctx   = (FnDestroyCtx)GetProcAddress  (dll, "mm_destroy_context");
    fn_analyze_file  = (FnAnalyzeFile)GetProcAddress (dll, "mm_analyze_file");
    fn_set_params    = (FnSetParams)GetProcAddress   (dll, "mm_set_params");
    fn_note_on       = (FnNoteOn)GetProcAddress      (dll, "mm_note_on");
    fn_note_off      = (FnNoteOff)GetProcAddress     (dll, "mm_note_off");
    fn_process       = (FnProcess)GetProcAddress     (dll, "mm_process");
    fn_log_message   = (FnLogMessage)GetProcAddress  (dll, "mm_log_message");

    bool ok = fn_create_ctx && fn_destroy_ctx && fn_analyze_file && fn_set_params && fn_note_on && fn_note_off && fn_process && fn_process && fn_log_message;
    if (!ok) {
        std::cerr << "RustBridge: failed to get all required symbols.�@DLL���ɕK�v�Ȋ֐������݂��܂���" << std::endl;
    }
    return ok;
}

/*
  DLL �ƃR���e�L�X�g�����S�ɔj������
*/
void RustBridge::unload()  //PluginProcessor.cpp�ŌĂ΂�Ă���
{
    if (ctx && fn_destroy_ctx) {
        fn_destroy_ctx(ctx);
        ctx = nullptr;
    }

    if (dll) {
        FreeLibrary(dll);
        dll = nullptr;
    }
}




/*
  Rust �ɃR���e�L�X�g����点�A�����ɕێ�����|�C���^�𓾂�
  - �߂�l true: ctx ������ꂽ
*/
bool RustBridge::create_context(float sample_rate, int block_size, int num_channels) //PrepareToPlay�ŌĂ΂�Ă���
{
    if (!fn_create_ctx) return false;
	ctx = fn_create_ctx(sample_rate, block_size, num_channels); //fn_create_ctx�ɂ�mm_create_context�̃A�h���X�������Ă���Bmm_create_context��Rust���Œ�`����Ă���֐�
    return ctx != nullptr;
}




void RustBridge::destroy_context()
{
    if (ctx && fn_destroy_ctx) {
        fn_destroy_ctx(ctx);
        ctx = nullptr;
    }
}

/*
  �t�@�C����́iblocking�j
  - �Ăяo�����Ńo�b�N�O���E���h�X���b�h���g������
*/
int RustBridge::analyzeFile(const char* path)
{
    if (!fn_analyze_file || !ctx) return -1;
    return fn_analyze_file(ctx, path);
}

/*
  �p�����[�^�̈ꊇ�ݒ�
  - ParamBundle �����̂܂� Rust �ɓn���iRust ���ŃR�s�[���������j
*/
void RustBridge::set_params(const ParamBundle& p)
{
    if (!fn_set_params || !ctx) return;
    fn_set_params(ctx, &p);
}

/*
  MIDI note on / off
*/
void RustBridge::note_on(int note, int velocity)
{
    if (!fn_note_on || !ctx) return;
    fn_note_on(ctx, note, velocity);
}

void RustBridge::note_off(int note)
{
    if (!fn_note_off || !ctx) return;
    fn_note_off(ctx, note);
}

/*
  �I�[�f�B�I�����Ăяo��
  - out_buffer_channel0: �`�����l��0 �̐��|�C���^�inum_samples ���j
  - Rust �͂��̗̈�� num_samples �̃T���v�����������ޑO��
  - �v���_�N�g�Ń}���`�`�����l���𐳊m�Ɉ����Ȃ�Afloat** �̕�����
    ���O�ɃC���^�[���[�u�o�b�t�@��p�ӂ�������ɕς��邱��
*/
void RustBridge::process(float* out_buffer_channel0, int num_samples, int num_channels)
{
    if (!fn_process || !ctx) return;
    fn_process(ctx, out_buffer_channel0, num_samples, num_channels);
}

void RustBridge::log(const std::string& message)
{
    if (fn_log_message) {
        fn_log_message(message.c_str());
    }
}