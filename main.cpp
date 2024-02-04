#include "KeyboardHook.h"
#include "TrayApplication.h"

#include <QResource>

int main(int argc, char *argv[])
{
    TrayApplication app(argc, argv);

    // 添加资源
    QResource::registerResource("myres.rcc");

    KeyboardHook keyboardHook;

    // 安装全局键盘事件钩子
    if (keyboardHook.installHook())
    {
        qDebug() << "Hook installed successfully.";
    }
    else
    {
        qDebug() << "Failed to install hook.";
        return 1;
    }

    return app.exec();
}
