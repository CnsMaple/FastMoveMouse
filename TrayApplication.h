#pragma once

#include "Setting.h"
#include <QApplication>
#include <QDialog>
#include <QMenu>
#include <QSystemTrayIcon>

class TrayApplication : public QApplication
{

        Q_OBJECT;

    public:
        TrayApplication(int argc, char *argv[]) : QApplication(argc, argv)
        {
            // 判断图标是否存在
            QIcon icon = QIcon("res/icon.png");
            if (icon.isNull())
            {
                icon = QIcon(":/res/icon.png");
            }

            // 创建系统托盘图标
            trayIcon = new QSystemTrayIcon(icon, this);

            // trayIcon = new QSystemTrayIcon(QIcon("../../res/icon.ico"), this);
            trayIcon->setToolTip("fish fast move mouse");

            // 创建托盘菜单
            QMenu *trayMenu = new QMenu();
            // 圆角
            // trayMenu->setObjectName("trayMenu");
            // trayMenu->setStyleSheet("#trayMenu { border-radius: 5px; }");
            // trayMenu->setStyleSheet("QMenu { border-radius: 10px; }"); // 设置圆角样式

            QAction *actionSetting = new QAction("Setting", this);
            QAction *actionExit = new QAction("Exit", this);

            connect(actionSetting, &QAction::triggered, this, &TrayApplication::showMainWindow);
            connect(actionExit, &QAction::triggered, this, &TrayApplication::exitApplication);

            trayMenu->addAction(actionSetting);
            trayMenu->addSeparator();
            trayMenu->addAction(actionExit);

            trayIcon->setContextMenu(trayMenu);

            setting = new Setting();
            config = &setting->config;

            // 显示托盘图标
            trayIcon->show();
        }

        void showMainWindow()
        {
            setting->show();
        }

        void exitApplication()
        {
            // 退出应用程序
            QApplication::exit();
        }

        static Setting *setting;

        static Config *config;

    private:
        QSystemTrayIcon *trayIcon;
};
