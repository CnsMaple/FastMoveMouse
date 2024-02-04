#pragma once
#include "ShowTipsWidget.h"
#include "TrayApplication.h"
#include "UiScan.h"
#include <QCoreApplication>
#include <QObject>
#include <QTimer>
#include <Windows.h>

class KeyboardHook : public QObject
{
        Q_OBJECT;

    public:
        KeyboardHook(QObject *parent = nullptr) : QObject(parent), hook(nullptr)
        {
            connect(timer, &QTimer::timeout, this, &KeyboardHook::moveMouse);
        }

        ~KeyboardHook()
        {
            uninstallHook();
        }

        bool installHook()
        {
            if (hook == nullptr)
            {
                hook = SetWindowsHookEx(WH_KEYBOARD_LL, KeyboardProc, GetModuleHandle(nullptr), 0);
                // 鼠标
                // hook = SetWindowsHookEx(WH_MOUSE_LL, KeyboardProc, GetModuleHandle(nullptr), 0);
                return (hook != nullptr);
            }
            return false;
        }

        void uninstallHook()
        {
            if (hook != nullptr)
            {
                UnhookWindowsHookEx(hook);
                hook = nullptr;
            }
        }

        static KeyboardHook *getInstance()
        {
            static KeyboardHook keyboardHook;
            return &keyboardHook;
        }

        void setKeyPressValue(int keyCode)
        {
            emit keyPressed(keyCode);
        }

        void setKeyReleaseValue(int keyCode)
        {
            emit keyReleased(keyCode);
        }

        void leftMouseClick()
        {
            mouse_event(MOUSEEVENTF_LEFTDOWN, 0, 0, 0, 0);
            mouse_event(MOUSEEVENTF_LEFTUP, 0, 0, 0, 0);
        }

        bool isLeftLong = false;

        void leftMouseClickPress()
        {
            isLeftLong = true;
            mouse_event(MOUSEEVENTF_LEFTDOWN, 0, 0, 0, 0);
        }

        void leftMouseClickRelease()
        {
            isLeftLong = false;
            mouse_event(MOUSEEVENTF_LEFTUP, 0, 0, 0, 0);
        }

        void middleMouseClick()
        {
            mouse_event(MOUSEEVENTF_MIDDLEDOWN, 0, 0, 0, 0);
            mouse_event(MOUSEEVENTF_MIDDLEUP, 0, 0, 0, 0);
        }

        bool isMiddleLong = false;

        void middleMouseClickPress()
        {
            isMiddleLong = true;
            mouse_event(MOUSEEVENTF_MIDDLEDOWN, 0, 0, 0, 0);
        }

        void middleMouseClickRelease()
        {
            isMiddleLong = false;
            mouse_event(MOUSEEVENTF_MIDDLEUP, 0, 0, 0, 0);
        }

        void rightMouseClick()
        {
            mouse_event(MOUSEEVENTF_RIGHTDOWN, 0, 0, 0, 0);
            mouse_event(MOUSEEVENTF_RIGHTUP, 0, 0, 0, 0);
        }

        bool isRightLong = false;

        void rightMouseClickPress()
        {
            isRightLong = true;
            mouse_event(MOUSEEVENTF_RIGHTDOWN, 0, 0, 0, 0);
        }

        void rightMouseClickRelease()
        {
            isRightLong = false;
            mouse_event(MOUSEEVENTF_RIGHTUP, 0, 0, 0, 0);
        }

        void moveMouse()
        {
            if (offsetX == 0 && offsetY == 0)
            {
                bool flag = false;
                int upOffset = TrayApplication::config->jsonMapInt["mouseMoveUpSize"];
                int downOffset = TrayApplication::config->jsonMapInt["mouseMoveDownSize"];
                int leftOffset = TrayApplication::config->jsonMapInt["mouseMoveLeftSize"];
                int rightOffset = TrayApplication::config->jsonMapInt["mouseMoveRightSize"];

                if (getInstance()->isMouseMoveAddSpeed)
                {
                    upOffset = TrayApplication::config->jsonMapInt["mouseMoveUpMaxSize"];
                    downOffset = TrayApplication::config->jsonMapInt["mouseMoveDownMaxSize"];
                    leftOffset = TrayApplication::config->jsonMapInt["mouseMoveLeftMaxSize"];
                    rightOffset = TrayApplication::config->jsonMapInt["mouseMoveRightMaxSize"];
                    // qDebug() << "set max speed";
                }
                if (getInstance()->isAllPressKey(
                        TrayApplication::config->jsonMapQList["mouseMoveUp"]))
                {
                    // qDebug() << "mouseMoveUp";
                    if (getInstance()->offsetY == 0)
                    {
                        getInstance()->offsetY -= upOffset;
                        // getInstance()->timer->start(1);
                    }
                    flag = true;
                }
                if (getInstance()->isAllPressKey(
                        TrayApplication::config->jsonMapQList["mouseMoveDown"]))
                {
                    // qDebug() << "mouseMoveDown";
                    if (getInstance()->offsetY == 0)
                    {
                        getInstance()->offsetY += downOffset;
                        // getInstance()->timer->start(1);
                    }
                    flag = true;
                }
                if (getInstance()->isAllPressKey(
                        TrayApplication::config->jsonMapQList["mouseMoveLeft"]))
                {
                    // qDebug() << "mouseMoveLeft";
                    if (getInstance()->offsetX == 0)
                    {
                        getInstance()->offsetX -= leftOffset;
                        // getInstance()->timer->start(1);
                    }
                    flag = true;
                }
                if (getInstance()->isAllPressKey(
                        TrayApplication::config->jsonMapQList["mouseMoveRight"]))
                {
                    // qDebug() << "mouseMoveRight";
                    if (getInstance()->offsetX == 0)
                    {
                        getInstance()->offsetX += rightOffset;
                        // getInstance()->timer->start(1);
                    }
                    flag = true;
                }
                if (!flag)
                {
                    timer->stop();
                }
                return;
            }

            int preOffsetX = offsetX;
            int preOffsetY = offsetY;

            if (offsetY != 0)
            {
                if (offsetY > 0)
                {
                    offsetY--;
                }
                else
                {
                    offsetY++;
                }
            }

            if (offsetX != 0)
            {
                if (offsetX > 0)
                {
                    offsetX--;
                }
                else
                {
                    offsetX++;
                }
            }

            INPUT input;
            input.type = INPUT_MOUSE;
            input.mi.dx = preOffsetX - offsetX;
            input.mi.dy = preOffsetY - offsetY;
            input.mi.mouseData = 0;
            input.mi.dwFlags = MOUSEEVENTF_MOVE;
            input.mi.time = 0;
            input.mi.dwExtraInfo = 0;

            SendInput(1, &input, sizeof(INPUT));
        }

        QTimer *timer = new QTimer(this);
        int offsetX = 0;
        int offsetY = 0;

    signals:
        void keyPressed(int keyCode);
        void keyReleased(int keyCode);

    private:
        HHOOK hook;

        ShowTipsWidget *showTipsWidget = new ShowTipsWidget();
        bool isShow = false;
        bool isMouseMoveAddSpeed = false;

        QList<bool> keyStatusList = QList<bool>(256, false);

        static LRESULT CALLBACK KeyboardProc(int nCode, WPARAM wParam, LPARAM lParam)
        {
            if (nCode == HC_ACTION)
            {
                auto keyCode = reinterpret_cast<KBDLLHOOKSTRUCT *>(lParam)->vkCode;
                // 鼠标
                // if (wParam == WM_LBUTTONDOWN)
                // {
                //     getInstance()->leftMouseClick();
                //     return 1;
                // }
                QString keyName = getInstance()->getKeyName(keyCode);
                if (wParam == WM_KEYDOWN || wParam == WM_SYSKEYDOWN)
                {
                    // qDebug() << "keyName press:" << keyName << "keyCode:" << keyCode;
                    getInstance()->keyStatusList[keyCode] = true;
                    bool flag = false;

                    if (TrayApplication::setting->isSetting)
                    {
                        qDebug() << "isSetting" << getInstance()->allPressKeyToString();
                        emit TrayApplication::setting->keyNameSend(
                            getInstance()->allPressKeyToString(),
                            getInstance()->getAllPressKey());
                        flag = true;
                    }
                    else if (getInstance()->isAllPressKey(
                                 TrayApplication::config->jsonMapQList["openJumpMouse"])
                             && !getInstance()->isShow)
                    {
                        HWND hwnd = GetForegroundWindow();

                        UiSCan ui_scan(hwnd);
                        QList<QRect> rects = ui_scan.getAllLeafRect();

                        getInstance()->isShow = true;
                        getInstance()->showTipsWidget->setRects(rects);
                        getInstance()->showTipsWidget->move(ui_scan.getRootRect().x(),
                                                            ui_scan.getRootRect().y());
                        getInstance()->showTipsWidget->resize(ui_scan.getRootRect().width(),
                                                              ui_scan.getRootRect().height());
                        getInstance()->showTipsWidget->show();
                        flag = true;
                    }
                    else if (getInstance()->isAllPressKey(
                                 TrayApplication::config->jsonMapQList["openJumpMouse"])
                             && getInstance()->isShow)
                    {
                        getInstance()->isShow = false;
                        getInstance()->showTipsWidget->hide();
                        flag = true;
                    }
                    else if (keyCode >= 65 && keyCode <= 90 && getInstance()->isShow)
                    {
                        if (getInstance()->showTipsWidget->removeOneRect(keyName))
                        {
                            getInstance()->isShow = false;
                            getInstance()->showTipsWidget->hide();
                        }
                        flag = true;
                    }
                    else if (getInstance()->isAllPressKey(
                                 TrayApplication::config->jsonMapQList["mouseLongLeft"]))
                    {
                        if (getInstance()->isLeftLong)
                        {
                            getInstance()->leftMouseClickRelease();
                        }
                        else
                        {
                            getInstance()->leftMouseClickPress();
                        }
                        flag = true;
                    }
                    else if (getInstance()->isAllPressKey(
                                 TrayApplication::config->jsonMapQList["mouseLongRight"]))
                    {
                        if (getInstance()->isRightLong)
                        {
                            getInstance()->rightMouseClickRelease();
                        }
                        else
                        {
                            getInstance()->rightMouseClickPress();
                        }
                        flag = true;
                    }
                    else if (getInstance()->isAllPressKey(
                                 TrayApplication::config->jsonMapQList["mouseLongMiddle"]))
                    {
                        if (getInstance()->isMiddleLong)
                        {
                            getInstance()->middleMouseClickRelease();
                        }
                        else
                        {
                            getInstance()->middleMouseClickPress();
                        }
                        flag = true;
                    }
                    else if (getInstance()->isAllPressKey(
                                 TrayApplication::config->jsonMapQList["mouseLeft"]))
                    {
                        getInstance()->leftMouseClick();
                        flag = true;
                    }
                    else if (getInstance()->isAllPressKey(
                                 TrayApplication::config->jsonMapQList["mouseMiddle"]))
                    {
                        getInstance()->middleMouseClick();
                        flag = true;
                    }
                    else if (getInstance()->isAllPressKey(
                                 TrayApplication::config->jsonMapQList["mouseRight"]))
                    {
                        getInstance()->rightMouseClick();
                        flag = true;
                    }
                    else if (getInstance()->isAllPressKey(
                                 TrayApplication::config->jsonMapQList["exitJumpMouse"]))
                    {
                        // 退出程序
                        QApplication::exit();
                        flag = true;
                        return 1;
                    }
                    else if (getInstance()->isAllPressKey(
                                 TrayApplication::config->jsonMapQList["mouseMoveAddSpeed"]))
                    {
                        getInstance()->isMouseMoveAddSpeed = true;
                        flag = true;
                    }

                    if (getInstance()->isAllPressKey(
                            TrayApplication::config->jsonMapQList["mouseMoveUp"])
                        || getInstance()->isAllPressKey(
                            TrayApplication::config->jsonMapQList["mouseMoveDown"])
                        || getInstance()->isAllPressKey(
                            TrayApplication::config->jsonMapQList["mouseMoveLeft"])
                        || getInstance()->isAllPressKey(
                            TrayApplication::config->jsonMapQList["mouseMoveRight"]))
                    {
                        getInstance()->timer->start(1);
                        flag = true;
                    }

                    if (flag)
                    {
                        return 1;
                    }
                }
                else if (wParam == WM_KEYUP || wParam == WM_SYSKEYUP)
                {
                    // qDebug() << "keyName release:" << keyName << "keyCode:" << keyCode;
                    getInstance()->keyStatusList[keyCode] = false;

                    if (TrayApplication::setting->isSetting && getInstance()->isNoPressKey())
                    {
                        qDebug() << "send end";
                        emit TrayApplication::setting->keyNameSendEnd();
                        getInstance()->isMouseMoveAddSpeed = false;
                    }

                    if (!getInstance()->isAllPressKey(
                            TrayApplication::config->jsonMapQList["mouseMoveAddSpeed"]))
                    {
                        getInstance()->isMouseMoveAddSpeed = false;
                    }
                }
            }

            return CallNextHookEx(nullptr, nCode, wParam, lParam);
        }

        QString getKeyName(int keyCode)
        {
            const int bufferSize = 256;
            wchar_t keyName[bufferSize];

            int result = GetKeyNameText(MapVirtualKey(keyCode, 0) << 16, keyName, bufferSize);

            if (result != 0)
            {
                // qDebug() << "now key name:" << QString::fromWCharArray(keyName) << keyCode;
                return QString::fromWCharArray(keyName);
            }
            else
            {
                // 处理错误情况
                qDebug() << "Failed to get key name";
                return QString();
            }
        }

        bool isAllPressKey(QList<int> needCheck)
        {
            for (auto i : needCheck)
            {
                if (!keyStatusList[i])
                {
                    return false;
                }
            }
            return true;
        }

        QList<int> getAllPressKey()
        {
            QList<int> result;
            for (int i = 0; i < keyStatusList.size(); i++)
            {
                if (keyStatusList[i])
                {
                    result.push_back(i);
                }
            }
            return result;
        }

        bool isNoPressKey()
        {
            for (auto i : keyStatusList)
            {
                if (i)
                {
                    return false;
                }
            }
            return true;
        }

        QString allPressKeyToString()
        {
            QString result;
            for (int i = 0; i < keyStatusList.size(); i++)
            {
                if (keyStatusList[i])
                {
                    result += getKeyName(i);
                    result += "+";
                }
            }
            return result.mid(0, result.length() - 1);
        }
};
