#include "Setting.h"

Setting::Setting(QWidget *parent) : QWidget(parent)
{

    // 判断图标是否存在
    QIcon icon = QIcon("res/icon.png");
    if (icon.isNull())
    {
        icon = QIcon(":/res/icon.png");
    }
    setWindowIcon(icon);
    layout = new QGridLayout();
    setLayout(layout);

    modeNameMapQList = {
        {"启动跳转鼠标",  "openJumpMouse"    },
        { "退出跳转鼠标", "exitJumpMouse"    },
        { "鼠标左键",       "mouseLeft"        },
        { "鼠标中键",       "mouseMiddle"      },
        { "鼠标右键",       "mouseRight"       },
        { "鼠标左键长按", "mouseLongLeft"    },
        { "鼠标中键长按", "mouseLongMiddle"  },
        { "鼠标右键长按", "mouseLongRight"   },
        { "鼠标上移",       "mouseMoveUp"      },
        { "鼠标下移",       "mouseMoveDown"    },
        { "鼠标左移",       "mouseMoveLeft"    },
        { "鼠标右移",       "mouseMoveRight"   },
        { "鼠标加速移动", "mouseMoveAddSpeed"},
    };

    modeNameMapInt = {
        {"鼠标上移速度",        "mouseMoveUpSize"      },
        { "鼠标下移速度",       "mouseMoveDownSize"    },
        { "鼠标左移速度",       "mouseMoveLeftSize"    },
        { "鼠标右移速度",       "mouseMoveRightSize"   },
        { "鼠标上移最大速度", "mouseMoveUpMaxSize"   },
        { "鼠标下移最大速度", "mouseMoveDownMaxSize" },
        { "鼠标左移最大速度", "mouseMoveLeftMaxSize" },
        { "鼠标右移最大速度", "mouseMoveRightMaxSize"},
    };

    generateConfigList();
}

void Setting::closeEvent(QCloseEvent *event)
{
    // 隐藏窗口而不是退出程序
    event->ignore();
    hide();
}
