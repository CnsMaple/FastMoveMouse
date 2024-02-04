#pragma once
#include <QDir>
#include <QFile>
#include <QJsonArray>
#include <QJsonDocument>
#include <QJsonObject>
#include <QMessageBox>
#include <QWidget>


class Config : public QWidget
{
        Q_OBJECT;

    public:
        Config(QWidget *parent = nullptr);
        ~Config() = default;

        bool loadConfig();
        bool saveConfig();

        void generateConfig(bool generateNew = false);

        QMap<QString, QList<int>> jsonMapQList = {
            {"openJumpMouse",    QList<int>()},
            { "exitJumpMouse",   QList<int>()},
            { "mouseLeft",       QList<int>()},
            { "mouseMiddle",     QList<int>()},
            { "mouseRight",      QList<int>()},
            { "mouseLongLeft",   QList<int>()},
            { "mouseLongMiddle", QList<int>()},
            { "mouseLongRight",  QList<int>()},
            { "mouseMoveUp",     QList<int>()},
            { "mouseMoveDown",   QList<int>()},
            { "mouseMoveLeft",   QList<int>()},
            { "mouseMoveRight",  QList<int>()},
            { "mouseMoveAddSpeed",  QList<int>()},
        };

        QMap<QString, int> jsonMapInt = {
            {"mouseMoveUpSize",        0},
            { "mouseMoveDownSize",     0},
            { "mouseMoveLeftSize",     0},
            { "mouseMoveRightSize",    0},
            { "mouseMoveUpMaxSize",    0},
            { "mouseMoveDownMaxSize",  0},
            { "mouseMoveLeftMaxSize",  0},
            { "mouseMoveRightMaxSize", 0},
        };

    private:
        QString path = "res/config.json";
};
