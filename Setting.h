#pragma once

#include "Config.h"
#include "qevent.h"
#include <QFile>
#include <QHBoxLayout>
#include <QJsonDocument>
#include <QJsonObject>
#include <QLabel>
#include <QLineEdit>
#include <QMessageBox>
#include <QPushButton>
#include <QVBoxLayout>
#include <QWidget>
#include <Windows.h>

class Setting : public QWidget
{
        Q_OBJECT;

    public:
        Setting(QWidget *parent = nullptr);
        ~Setting() = default;
        void closeEvent(QCloseEvent *event) override;
        bool isSetting = false;
        Config config;
        QMap<QString, QString> modeNameMapQList;
        QMap<QString, QString> modeNameMapInt;

        QString getKeyName(int keyCode)
        {
            const int bufferSize = 256;
            wchar_t keyName[bufferSize];

            int result = GetKeyNameText(MapVirtualKey(keyCode, 0) << 16, keyName, bufferSize);

            if (result != 0)
            {
                // qDebug() << "now key name:" << QString::fromWCharArray(keyName);
                return QString::fromWCharArray(keyName);
            }
            else
            {
                // 处理错误情况
                qDebug() << "Failed to get key name";
                return QString();
            }
        }

        QString allPressKeyToString(QList<int> keyStatusList)
        {
            QString result;
            for (int i : keyStatusList)
            {
                result += getKeyName(i);
                result += "+";
            }
            return result.mid(0, result.length() - 1);
        }

        void generateConfigList()
        {
            int i = 0;
            // 清空layout
            QLayoutItem *child;
            while ((child = layout->takeAt(0)) != nullptr)
            {
                delete child->widget(); // 删除widget
                delete child;           // 删除QLayoutItem
            }

            for (auto it = modeNameMapQList.begin(); it != modeNameMapQList.end(); it++)
            {

                QString modeName = it.key();
                QString keyName = it.value();
                QLabel *titleLabel = new QLabel(modeName + "：", this);
                QList<int> *keyListP = &config.jsonMapQList[keyName];
                QLabel *titleLabel1 = new QLabel(allPressKeyToString(*keyListP), this);
                QPushButton *button = new QPushButton("检测按键", this);
                connect(button,
                        &QPushButton::clicked,
                        [=]()
                        {
                            // button1->setText("检测中");
                            // isSetting = true;
                            QString buttonName = button->text();
                            if (buttonName == "检测按键")
                            {
                                emit keyNameSendEnd();
                                button->setText("检测中");
                                isSetting = true;
                                nowModeName = modeName;
                            }
                            else if (buttonName == "检测中")
                            {
                                button->setText("检测按键");
                                isSetting = false;
                                nowModeName = "";
                            }
                        });
                connect(this,
                        &Setting::keyNameSend,
                        [=](QString keyName, QList<int> keyList)
                        {
                            if (nowModeName
                                == titleLabel->text().mid(0, titleLabel->text().length() - 1))
                            {
                                titleLabel1->setText(keyName);
                                *keyListP = keyList;
                            }
                        });
                connect(this,
                        &Setting::keyNameSendEnd,
                        [=]()
                        {
                            button->setText("检测按键");
                            isSetting = false;
                            nowModeName = "";
                        });
                layout->addWidget(titleLabel, i, 0);
                layout->addWidget(titleLabel1, i, 1);
                layout->addWidget(button, i, 2);
                i++;
            }

            i = 0;
            for (auto it = modeNameMapInt.begin(); it != modeNameMapInt.end(); it++)
            {
                QString modeName = it.key();
                QString keyName = it.value();
                QLabel *titleLabel = new QLabel(modeName + "：", this);
                QLineEdit *lineEdit = new QLineEdit(this);
                // lineEdit->setPlaceholderText(keyName);
                lineEdit->setText(QString::number(config.jsonMapInt[keyName]));
                layout->addWidget(titleLabel, i, 3);
                layout->addWidget(lineEdit, i, 4);
                i++;
            }

            QPushButton *button3 = new QPushButton("保存设置", this);
            connect(
                button3,
                &QPushButton::clicked,
                [=]()
                {
                    // 读取布局中的lineEdit
                    for (int i = 0; i < modeNameMapInt.size(); i++)
                    {
                        QLabel *label =
                            static_cast<QLabel *>(layout->itemAtPosition(i, 3)->widget());
                        QLineEdit *lineEdit =
                            static_cast<QLineEdit *>(layout->itemAtPosition(i, 4)->widget());
                        if (label == nullptr || lineEdit == nullptr)
                        {
                            continue;
                        }
                        // 获取数字,并判断异常捕获
                        int value = 0;
                        bool ok = false;
                        value = lineEdit->text().toInt(&ok);
                        if (ok)
                        {
                            config.jsonMapInt
                                [modeNameMapInt[label->text().mid(0, label->text().length() - 1)]] =
                                value;
                        }
                        else
                        {
                            QMessageBox::critical(this, "错误", "数字输入异常");
                            return;
                        }
                    }

                    config.saveConfig();
                    QMessageBox::information(this, "提示", "保存成功");
                });
            layout->addWidget(button3, 100, 3);

            QPushButton *button4 = new QPushButton("恢复默认设置", this);
            connect(button4,
                    &QPushButton::clicked,
                    [=]()
                    {
                        config.generateConfig(true);
                        generateConfigList();
                        QMessageBox::information(this, "提示", "恢复成功");
                    });
            layout->addWidget(button4, 100, 4);
        }

    signals:
        void keyNameSend(QString keyName, QList<int> keyList);
        void keyNameSendEnd();

    private:
        QString nowModeName;
        QGridLayout *layout;
};
